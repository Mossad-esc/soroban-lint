//! SOR008 `std-or-float-usage`
//!
//! Detects Soroban contract code that uses the standard library or
//! floating-point types, both of which are incompatible with Soroban's
//! WebAssembly execution environment.
//!
//! # What is flagged
//!
//! 1. **Missing `#![no_std]`** at the crate root (`lib.rs` of a contract
//!    crate, as identified by the `ContractModel.is_contract()` flag). Emits
//!    one finding at the top of the file.  The `#![cfg_attr(not(test),
//!    no_std)]` form is treated as present (the model's `no_std` flag handles
//!    this).
//!
//! 2. **`use std::…` paths** outside of `#[cfg(test)]` code — imports from
//!    the standard library.
//!
//! 3. **`std::` qualified paths** anywhere in non-test code, for example
//!    `std::collections::HashMap`.
//!
//! 4. **`f32` or `f64` types** anywhere in non-test code (in function
//!    signatures, struct fields, or local variable type annotations).
//!
//! 5. **Float literals** such as `1.0`, `3.14f32`, etc., in non-test code.
//!
//! # What is NOT flagged
//!
//! - Code inside `#[cfg(test)]` modules or functions.
//! - Non-contract crates (no `#[contract]` struct and no `#[contractimpl]`
//!   block). The rule is a no-op on such files.
//!
//! # References
//!
//! - README: [SOR008 · std-or-float-usage](../../../../../README.md#sor008--std-or-float-usage)

use syn::visit::Visit;
use syn::{Expr, ExprLit, ItemUse, Lit, TypePath, UseTree};

use lint_core::{util::has_cfg_test, Finding, Rule, RuleContext, Severity};

const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR008.md";

/// Implementation of the `std-or-float-usage` rule.
#[derive(Debug, Default)]
pub struct Sor008;

impl Rule for Sor008 {
    fn id(&self) -> &'static str {
        "SOR008"
    }

    fn name(&self) -> &'static str {
        "std-or-float-usage"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        // Only analyze contract crates.
        if !ctx.model().is_contract() {
            return Vec::new();
        }

        let severity = ctx.config().severity_or(self.default_severity());
        let mut findings = Vec::new();

        // 1. Missing #![no_std].
        // The `no_std` inner attribute is crate-level: it only needs to appear
        // in `lib.rs` (or `main.rs`), not in every sub-module.  Emitting this
        // finding on a sub-module such as `contract.rs` or `test.rs` is a
        // false positive confirmed in corpus triage (soroban-examples token
        // contract, and three test-only files).
        let is_root = ctx
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "lib.rs" || n == "main.rs");

        if is_root && !ctx.model().no_std {
            findings.push(
                Finding::new(
                    "SOR008",
                    "std-or-float-usage",
                    severity,
                    "contract crate is missing `#![no_std]`; \
                     Soroban contracts compile to WebAssembly and must not use \
                     the standard library",
                    ctx.path(),
                    1,
                    1,
                )
                .with_help(
                    "add `#![no_std]` (or `#![cfg_attr(not(test), no_std)]`) \
                     at the top of lib.rs",
                )
                .with_docs_url(DOCS_URL),
            );
        }

        // 2–5. Walk the AST for std paths and float usage.
        let mut visitor = Sor008Visitor {
            findings: &mut findings,
            path: ctx.path(),
            severity,
            in_cfg_test: false,
        };
        visitor.visit_file(ctx.ast());

        findings
    }
}

// ─── Visitor ─────────────────────────────────────────────────────────────────

struct Sor008Visitor<'a> {
    findings: &'a mut Vec<Finding>,
    path: &'a std::path::Path,
    severity: Severity,
    in_cfg_test: bool,
}

impl<'a, 'ast> Visit<'ast> for Sor008Visitor<'a> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs: Option<&Vec<syn::Attribute>> = match item {
            syn::Item::Fn(f) => Some(&f.attrs),
            syn::Item::Mod(m) => Some(&m.attrs),
            _ => None,
        };
        if let Some(attrs) = attrs {
            if has_cfg_test(attrs) {
                // Skip test items entirely.
                return;
            }
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_item_use(&mut self, use_item: &'ast ItemUse) {
        if self.in_cfg_test {
            return;
        }
        if has_cfg_test(&use_item.attrs) {
            return;
        }
        // Check if the use tree starts with `std`.
        if use_tree_starts_with_std(&use_item.tree) {
            let (line, col) = lint_core::util::span_location(use_item.use_token.span);
            self.emit(
                line,
                col,
                "`use std::…` imports from the standard library; \
                 use `soroban_sdk` types instead",
            );
        }
        syn::visit::visit_item_use(self, use_item);
    }

    fn visit_type_path(&mut self, type_path: &'ast TypePath) {
        if self.in_cfg_test {
            return;
        }
        let path = &type_path.path;

        // f32 or f64 as a type.
        if let Some(ident) = path.get_ident() {
            let name = ident.to_string();
            if name == "f32" || name == "f64" {
                let (line, col) = lint_core::util::span_location(ident.span());
                self.emit(
                    line,
                    col,
                    &format!(
                        "`{name}` is a floating-point type; \
                         use fixed-point integer math with explicit scaling instead"
                    ),
                );
            }
        }

        // std:: qualified path.
        if let Some(first) = path.segments.first() {
            if first.ident == "std" {
                let (line, col) = lint_core::util::span_location(first.ident.span());
                self.emit(
                    line,
                    col,
                    "`std::` path found; Soroban contracts must not use the standard library",
                );
            }
        }

        syn::visit::visit_type_path(self, type_path);
    }

    fn visit_expr_lit(&mut self, lit: &'ast ExprLit) {
        if self.in_cfg_test {
            return;
        }
        if let Lit::Float(_) = &lit.lit {
            let (line, col) = lit_float_span(&lit.lit);
            self.emit(
                line,
                col,
                "float literal found; use fixed-point integer math \
                 with explicit scaling and rounding instead",
            );
        }
        syn::visit::visit_expr_lit(self, lit);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        if self.in_cfg_test {
            return;
        }
        // Check for `std::` as a path expression.
        if let Expr::Path(ep) = expr {
            if let Some(first) = ep.path.segments.first() {
                if first.ident == "std" {
                    let (line, col) = lint_core::util::span_location(first.ident.span());
                    self.emit(
                        line,
                        col,
                        "`std::` path found; Soroban contracts must not use the standard library",
                    );
                }
            }
        }
        syn::visit::visit_expr(self, expr);
    }
}

impl<'a> Sor008Visitor<'a> {
    fn emit(&mut self, line: usize, col: usize, message: &str) {
        self.findings.push(
            Finding::new(
                "SOR008",
                "std-or-float-usage",
                self.severity,
                message,
                self.path,
                line,
                col,
            )
            .with_docs_url(DOCS_URL),
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Returns the 1-based line/column of a float literal.
fn lit_float_span(lit: &Lit) -> (usize, usize) {
    match lit {
        Lit::Float(f) => lint_core::util::span_location(f.span()),
        _ => (1, 1),
    }
}

/// Returns true when the `UseTree` starts with the segment `std`.
fn use_tree_starts_with_std(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path(path) => path.ident == "std",
        UseTree::Name(name) => name.ident == "std",
        UseTree::Rename(r) => r.ident == "std",
        UseTree::Group(g) => g.items.iter().any(use_tree_starts_with_std),
        UseTree::Glob(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::{ContractModel, RuleConfig, RuleContext};
    use syn::parse_quote;

    fn run(file: syn::File) -> Vec<Finding> {
        // Use "lib.rs" so the missing-no_std check fires (it only fires on
        // crate-root files to avoid false positives on sub-modules).
        let path = std::path::Path::new("lib.rs");
        let model = ContractModel::build(&file);
        let config = RuleConfig::default();
        let ctx = RuleContext::new(&file, path, "", &model, &config);
        Sor008.check(&ctx)
    }

    #[test]
    fn flags_missing_no_std() {
        let file: syn::File = parse_quote! {
            #[contract]
            pub struct Token;

            #[contractimpl]
            impl Token {
                pub fn hello(env: Env) {}
            }
        };
        let findings = run(file);
        assert!(findings.iter().any(|f| f.message.contains("no_std")));
    }

    #[test]
    fn does_not_flag_with_no_std() {
        let file: syn::File = parse_quote! {
            #![no_std]

            #[contract]
            pub struct Token;

            #[contractimpl]
            impl Token {
                pub fn hello(env: Env) {}
            }
        };
        let findings = run(file);
        assert!(!findings.iter().any(|f| f.message.contains("no_std")));
    }

    #[test]
    fn flags_f64_type() {
        let file: syn::File = parse_quote! {
            #![no_std]
            #[contract]
            pub struct Token;

            #[contractimpl]
            impl Token {
                pub fn ratio(env: Env, a: i128, b: i128) -> f64 {
                    1.0
                }
            }
        };
        let findings = run(file);
        assert!(findings.iter().any(|f| f.message.contains("f64")));
    }

    #[test]
    fn does_not_flag_non_contract() {
        // A non-contract file (helper library, etc.) is not analyzed.
        let file: syn::File = parse_quote! {
            pub fn helper() -> f64 { 1.0 }
        };
        assert!(run(file).is_empty());
    }
}
