//! The structural model of a contract extracted from a parsed file.
//!
//! The model is step 3 of the README's "How It Works": it identifies
//! `#[contract]` structs, `#[contractimpl]` blocks, public entrypoints,
//! `#[contracttype]` / `#[contracterror]` definitions, private helpers, and
//! whether the file opts into `#![no_std]`. The model borrows the parsed AST,
//! so it is cheap to build and shares spans with the source.

use syn::{
    Attribute, Block, FnArg, ImplItem, Item, ItemEnum, ItemImpl, ItemStruct, Pat, PatType,
    ReturnType, Signature, Type, Visibility,
};

use crate::util::{attr_name, attr_tokens_contain, has_attr, span_location};

/// A function-shaped item: an entrypoint or a hook in the call graph.
#[derive(Debug)]
pub struct Function<'ast> {
    /// The function name.
    pub name: String,
    /// The full signature.
    pub signature: &'ast Signature,
    /// The function body.
    pub block: &'ast Block,
    /// 1-based line of the function identifier.
    pub line: usize,
    /// 1-based column of the function identifier.
    pub column: usize,
}

impl<'ast> Function<'ast> {
    /// The parameter names, excluding any `self` receiver.
    #[must_use]
    pub fn param_names(&self) -> Vec<String> {
        self.signature
            .inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Typed(PatType { pat, .. }) => pattern_ident(pat),
                FnArg::Receiver(_) => None,
            })
            .collect()
    }

    /// The return type, when the function returns a value.
    #[must_use]
    pub fn return_type(&self) -> Option<&'ast Type> {
        match &self.signature.output {
            ReturnType::Type(_, ty) => Some(ty),
            ReturnType::Default => None,
        }
    }
}

/// A public function inside a `#[contractimpl]` block.
pub type Entrypoint<'ast> = Function<'ast>;

/// A private function or associated function reachable from entrypoints.
pub type HelperFn<'ast> = Function<'ast>;

/// A `#[contract]` struct.
#[derive(Debug)]
pub struct ContractStruct<'ast> {
    /// The struct name.
    pub name: String,
    /// 1-based line of the struct identifier.
    pub line: usize,
    /// 1-based column of the struct identifier.
    pub column: usize,
    /// The underlying item.
    pub item: &'ast ItemStruct,
}

/// A `#[contractimpl]` impl block.
#[derive(Debug)]
pub struct ContractImpl<'ast> {
    /// The implemented type, for example `Token`.
    pub self_ty: String,
    /// Public functions, in source order.
    pub entrypoints: Vec<Entrypoint<'ast>>,
    /// Private associated functions, in source order.
    pub helpers: Vec<HelperFn<'ast>>,
    /// 1-based line of the impl block.
    pub line: usize,
    /// 1-based column of the impl block.
    pub column: usize,
}

/// The kind of a `#[contracttype]` item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractTypeKind {
    /// A struct.
    Struct,
    /// An enum.
    Enum,
    /// A union.
    Union,
}

/// A `#[contracttype]` item.
#[derive(Debug)]
pub struct ContractType<'ast> {
    /// The type name.
    pub name: String,
    /// Whether the item is a struct, enum, or union.
    pub kind: ContractTypeKind,
    /// Enum variants (empty for structs and unions).
    pub variants: Vec<EnumVariant>,
    /// 1-based line of the type identifier.
    pub line: usize,
    /// 1-based column of the type identifier.
    pub column: usize,
    /// The underlying item, when it is a struct.
    pub struct_item: Option<&'ast ItemStruct>,
    /// The underlying item, when it is an enum.
    pub enum_item: Option<&'ast ItemEnum>,
}

/// A `#[contracterror]` enum.
#[derive(Debug)]
pub struct ContractError<'ast> {
    /// The error enum name.
    pub name: String,
    /// The declared variants.
    pub variants: Vec<EnumVariant>,
    /// 1-based line of the enum identifier.
    pub line: usize,
    /// 1-based column of the enum identifier.
    pub column: usize,
    /// The underlying item.
    pub item: &'ast ItemEnum,
}

/// A single enum variant.
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// The variant name.
    pub name: String,
    /// 1-based line of the variant identifier.
    pub line: usize,
    /// 1-based column of the variant identifier.
    pub column: usize,
}

/// Structural model of a Soroban contract.
#[derive(Debug, Default)]
pub struct ContractModel<'ast> {
    /// `#[contract]` structs.
    pub contracts: Vec<ContractStruct<'ast>>,
    /// `#[contractimpl]` blocks.
    pub impls: Vec<ContractImpl<'ast>>,
    /// `#[contracttype]` items.
    pub types: Vec<ContractType<'ast>>,
    /// `#[contracterror]` enums.
    pub errors: Vec<ContractError<'ast>>,
    /// Free private functions.
    pub helpers: Vec<HelperFn<'ast>>,
    /// Whether the file opts into `#![no_std]`.
    pub no_std: bool,
}

impl<'ast> ContractModel<'ast> {
    /// Builds a model from a parsed file.
    #[must_use]
    pub fn build(file: &'ast syn::File) -> Self {
        let mut model = Self {
            no_std: file.attrs.iter().any(is_no_std_attr),
            ..Self::default()
        };
        for item in &file.items {
            model.collect_item(item);
        }
        model
    }

    /// Whether the file looks like a contract crate (has a contract struct or
    /// an impl block).
    #[must_use]
    pub fn is_contract(&self) -> bool {
        !self.contracts.is_empty() || !self.impls.is_empty()
    }

    /// Iterates over every public entrypoint across all impl blocks.
    pub fn entrypoints(&self) -> impl Iterator<Item = &Entrypoint<'ast>> {
        self.impls.iter().flat_map(|imp| imp.entrypoints.iter())
    }

    /// The names of every helper function, including impl helpers.
    #[must_use]
    pub fn helper_names(&self) -> Vec<&str> {
        self.helpers
            .iter()
            .chain(self.impls.iter().flat_map(|imp| imp.helpers.iter()))
            .map(|helper| helper.name.as_str())
            .collect()
    }

    /// The names of every `#[contracterror]` enum.
    #[must_use]
    pub fn error_enum_names(&self) -> Vec<&str> {
        self.errors
            .iter()
            .map(|error| error.name.as_str())
            .collect()
    }

    fn collect_item(&mut self, item: &'ast Item) {
        match item {
            Item::Struct(item_struct) => self.collect_struct(item_struct),
            Item::Enum(item_enum) => self.collect_enum(item_enum),
            Item::Impl(item_impl) => self.collect_impl(item_impl),
            Item::Fn(item_fn) if !matches!(item_fn.vis, Visibility::Public(_)) => {
                self.helpers
                    .push(function_from_signature(&item_fn.sig, &item_fn.block));
            }
            _ => {}
        }
    }

    fn collect_struct(&mut self, item: &'ast ItemStruct) {
        let (line, column) = span_location(item.ident.span());
        if has_attr(&item.attrs, "contract") {
            self.contracts.push(ContractStruct {
                name: item.ident.to_string(),
                line,
                column,
                item,
            });
        } else if has_attr(&item.attrs, "contracttype") {
            self.types.push(ContractType {
                name: item.ident.to_string(),
                kind: ContractTypeKind::Struct,
                variants: Vec::new(),
                line,
                column,
                struct_item: Some(item),
                enum_item: None,
            });
        }
    }

    fn collect_enum(&mut self, item: &'ast ItemEnum) {
        let (line, column) = span_location(item.ident.span());
        let variants = enum_variants(item);
        if has_attr(&item.attrs, "contracttype") {
            self.types.push(ContractType {
                name: item.ident.to_string(),
                kind: ContractTypeKind::Enum,
                variants,
                line,
                column,
                struct_item: None,
                enum_item: Some(item),
            });
        } else if has_attr(&item.attrs, "contracterror") {
            self.errors.push(ContractError {
                name: item.ident.to_string(),
                variants,
                line,
                column,
                item,
            });
        }
    }

    fn collect_impl(&mut self, item: &'ast ItemImpl) {
        let is_contract_impl = has_attr(&item.attrs, "contractimpl");
        let self_ty = type_name(&item.self_ty);
        let (line, column) = span_location(item.impl_token.span);

        let mut entrypoints = Vec::new();
        let mut impl_helpers = Vec::new();

        for impl_item in &item.items {
            let ImplItem::Fn(method) = impl_item else {
                continue;
            };
            let function = function_from_signature(&method.sig, &method.block);
            if is_contract_impl && matches!(method.vis, Visibility::Public(_)) {
                entrypoints.push(function);
            } else if !matches!(method.vis, Visibility::Public(_)) {
                impl_helpers.push(function);
            }
        }

        if is_contract_impl {
            self.impls.push(ContractImpl {
                self_ty,
                entrypoints,
                helpers: impl_helpers,
                line,
                column,
            });
        } else {
            self.helpers.extend(impl_helpers);
        }
    }
}

/// Builds a [`Function`] from a signature and body.
fn function_from_signature<'ast>(signature: &'ast Signature, block: &'ast Block) -> Function<'ast> {
    let (line, column) = span_location(signature.ident.span());
    Function {
        name: signature.ident.to_string(),
        signature,
        block,
        line,
        column,
    }
}

/// Extracts enum variant names and locations.
fn enum_variants(item: &ItemEnum) -> Vec<EnumVariant> {
    item.variants
        .iter()
        .map(|variant| {
            let (line, column) = span_location(variant.ident.span());
            EnumVariant {
                name: variant.ident.to_string(),
                line,
                column,
            }
        })
        .collect()
}

/// Renders the implemented type name, for example `Token`.
fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map_or_else(|| "?".to_owned(), |segment| segment.ident.to_string()),
        _ => "?".to_owned(),
    }
}

/// Returns the name of an identifier pattern, for example a parameter name.
fn pattern_ident(pat: &Pat) -> Option<String> {
    match pat {
        Pat::Ident(ident) => Some(ident.ident.to_string()),
        _ => None,
    }
}

/// Whether an attribute opts the file into `#![no_std]`, including the
/// `#![cfg_attr(not(test), no_std)]` form.
fn is_no_std_attr(attr: &Attribute) -> bool {
    match attr_name(attr).as_deref() {
        Some("no_std") => true,
        Some("cfg_attr") => attr_tokens_contain(attr, "no_std"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn fixture(name: &str) -> syn::File {
        let path = format!(
            "{}/../../tests/fixtures/safe/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let source =
            std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path}: {err}"));
        syn::parse_file(&source).expect("fixture should parse")
    }

    fn names<'a>(functions: impl Iterator<Item = &'a str>) -> Vec<String> {
        functions.map(str::to_owned).collect()
    }

    #[test]
    fn detects_minimal_token() {
        let file = fixture("minimal_token.rs");
        let model = ContractModel::build(&file);

        assert!(model.no_std);
        assert!(model.is_contract());
        assert_eq!(model.contracts.len(), 1);
        assert_eq!(model.contracts[0].name, "Token");

        assert_eq!(model.impls.len(), 1);
        assert_eq!(model.impls[0].self_ty, "Token");
        let entrypoints: Vec<&str> = model.entrypoints().map(|e| e.name.as_str()).collect();
        assert_eq!(entrypoints, vec!["initialize", "balance", "transfer"]);

        let transfer = model
            .entrypoints()
            .find(|e| e.name == "transfer")
            .expect("transfer entrypoint");
        assert_eq!(transfer.param_names(), vec!["env", "from", "to", "amount"]);
        assert_eq!(transfer.return_type(), None);
        assert!(transfer.line > 1);

        assert_eq!(model.helper_names(), vec!["read_balance"]);

        assert_eq!(model.error_enum_names(), vec!["Error"]);
        assert_eq!(
            model.errors[0]
                .variants
                .iter()
                .map(|variant| variant.name.as_str())
                .collect::<Vec<_>>(),
            vec!["NotInitialized", "AlreadyInitialized"]
        );

        let data_key = model
            .types
            .iter()
            .find(|ty| ty.name == "DataKey")
            .expect("DataKey type");
        assert_eq!(data_key.kind, ContractTypeKind::Enum);
        assert_eq!(
            data_key
                .variants
                .iter()
                .map(|variant| variant.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Admin", "Balance"]
        );
    }

    #[test]
    fn detects_qualified_attribute_paths() {
        let file: syn::File = parse_quote! {
            #[soroban_sdk::contract]
            pub struct Counter;

            #[soroban_sdk::contractimpl]
            impl Counter {
                pub fn increment(env: soroban_sdk::Env) {}
                fn helper() {}
            }
        };
        let model = ContractModel::build(&file);
        assert_eq!(model.contracts[0].name, "Counter");
        assert_eq!(model.impls[0].self_ty, "Counter");
        assert_eq!(
            names(
                model
                    .entrypoints()
                    .map(|entrypoint| entrypoint.name.as_str())
            ),
            vec!["increment"]
        );
        assert_eq!(model.helper_names(), vec!["helper"]);
    }

    #[test]
    fn detects_contractimpl_with_arguments() {
        let file: syn::File = parse_quote! {
            #[contractimpl(export = true)]
            impl Token {
                pub fn transfer(env: Env) {}
            }
        };
        let model = ContractModel::build(&file);
        assert_eq!(model.impls.len(), 1);
        assert_eq!(
            names(
                model
                    .entrypoints()
                    .map(|entrypoint| entrypoint.name.as_str())
            ),
            vec!["transfer"]
        );
    }

    #[test]
    fn detects_no_std_cfg_attr_form() {
        let file: syn::File = parse_quote! {
            #![cfg_attr(not(test), no_std)]
            pub fn f() {}
        };
        let model = ContractModel::build(&file);
        assert!(model.no_std);
    }

    #[test]
    fn non_contract_impl_helpers_are_collected() {
        let file: syn::File = parse_quote! {
            impl Helpers {
                fn helper() {}
                pub fn public() {}
            }
        };
        let model = ContractModel::build(&file);
        assert!(model.impls.is_empty());
        assert_eq!(model.helper_names(), vec!["helper"]);
    }

    #[test]
    fn only_private_free_fns_are_helpers() {
        let file: syn::File = parse_quote! {
            pub fn public() {}
            fn private() {}
        };
        let model = ContractModel::build(&file);
        assert_eq!(model.helper_names(), vec!["private"]);
    }
}
