//! Shared helpers for working with `syn` trees.
//!
//! These utilities are used by the contract model and, later, by rules. They
//! deliberately operate on the pre-expansion syntax tree, so they match
//! attribute paths by their final segment (for example
//! `soroban_sdk::contractimpl` matches `contractimpl`).

use proc_macro2::Span;
use syn::{Attribute, Expr, Meta};

/// Returns the final segment of an attribute path, for example `contractimpl`
/// for `soroban_sdk::contractimpl`.
#[must_use]
pub fn attr_name(attr: &Attribute) -> Option<String> {
    attr.path()
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}

/// Whether any attribute's final path segment equals `name`.
#[must_use]
pub fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    find_attr(attrs, name).is_some()
}

/// Finds the first attribute whose final path segment equals `name`.
#[must_use]
pub fn find_attr<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
    attrs
        .iter()
        .find(|attr| attr_name(attr).as_deref() == Some(name))
}

/// Whether the textual form of a list-style attribute contains `needle`.
///
/// Used for forms such as `#[cfg_attr(not(test), no_std)]` and `#[cfg(test)]`.
/// Only the argument list is inspected, which is where `cfg`/`cfg_attr`
/// conditions appear.
#[must_use]
pub fn attr_tokens_contain(attr: &Attribute, needle: &str) -> bool {
    match &attr.meta {
        Meta::List(list) => list.tokens.to_string().contains(needle),
        Meta::Path(_) | Meta::NameValue(_) => false,
    }
}

/// Whether the attributes contain `#[cfg(test)]` or another `cfg` mentioning
/// `test`.
#[must_use]
pub fn has_cfg_test(attrs: &[Attribute]) -> bool {
    find_attr(attrs, "cfg").is_some_and(|attr| attr_tokens_contain(attr, "test"))
}

/// Returns the 1-based line and column of a span.
#[must_use]
pub fn span_location(span: Span) -> (usize, usize) {
    let start = span.start();
    (start.line, start.column + 1)
}

/// Returns `true` when `expr` is a method-call chain that contains a call to
/// `name` anywhere in its receiver chain.
///
/// This is the walker used by the README's SOR001 approach for recognizing
/// `env.storage().persistent().set(...)`-style chains.
#[must_use]
pub fn chain_contains(expr: &Expr, name: &str) -> bool {
    let mut current = expr;
    loop {
        match current {
            Expr::MethodCall(call) => {
                if call.method == name {
                    return true;
                }
                current = &call.receiver;
            }
            Expr::Field(field) => current = &field.base,
            Expr::Try(inner) => current = &inner.expr,
            Expr::Paren(inner) => current = &inner.expr,
            Expr::Group(inner) => current = &inner.expr,
            Expr::Reference(inner) => current = &inner.expr,
            _ => return false,
        }
    }
}

/// Returns the name of the outermost method call in `expr`, if it is one.
#[must_use]
pub fn outer_method_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::MethodCall(call) => Some(call.method.to_string()),
        Expr::Try(inner) => outer_method_name(&inner.expr),
        Expr::Paren(inner) => outer_method_name(&inner.expr),
        Expr::Group(inner) => outer_method_name(&inner.expr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn matches_attribute_by_final_segment() {
        let item: syn::ItemFn = parse_quote! {
            #[soroban_sdk::contractimpl]
            fn f() {}
        };
        assert!(has_attr(&item.attrs, "contractimpl"));
        assert!(!has_attr(&item.attrs, "contract"));
    }

    #[test]
    fn detects_cfg_test() {
        let item: syn::ItemFn = parse_quote! {
            #[cfg(test)]
            fn f() {}
        };
        assert!(has_cfg_test(&item.attrs));
    }

    #[test]
    fn detects_no_std_in_cfg_attr() {
        let file: syn::File = parse_quote! {
            #![cfg_attr(not(test), no_std)]
            fn f() {}
        };
        assert!(attr_tokens_contain(&file.attrs[0], "no_std"));
    }

    #[test]
    fn chain_walker_finds_nested_method() {
        let expr: Expr = parse_quote! {
            env.storage().persistent().set(&key, &value)
        };
        assert!(chain_contains(&expr, "storage"));
        assert!(chain_contains(&expr, "persistent"));
        assert!(chain_contains(&expr, "set"));
        assert!(!chain_contains(&expr, "temporary"));
        assert_eq!(outer_method_name(&expr).as_deref(), Some("set"));
    }

    #[test]
    fn span_location_is_one_based() {
        let item: syn::ItemFn = parse_quote! {
            fn f() {}
        };
        let (line, column) = span_location(item.sig.ident.span());
        assert_eq!(line, 1);
        assert!(column >= 1);
    }
}
