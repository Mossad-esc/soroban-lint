//! SOR006 `panic-in-entrypoint`
//!
//! Detects panic-producing patterns in public entrypoints and helpers
//! reachable from them via the call graph.
//!
//! # What is flagged
//!
//! Inside any public entrypoint (or a helper it calls):
//!
//! - `.unwrap()` method calls
//! - `.expect(…)` method calls
//! - `panic!(…)` macro invocations
//! - `unreachable!(…)` macro invocations
//! - `todo!(…)` macro invocations
//! - `unimplemented!(…)` macro invocations
//! - Direct indexing with a non-constant index expression: `vec[idx]`
//!
//! # What is NOT flagged
//!
//! - `unwrap_or(…)`, `unwrap_or_else(…)`, `unwrap_or_default()` — these are
//!   safe alternatives that do not panic.
//! - `?` (the Try operator) — this propagates a typed error and is the
//!   recommended pattern.
//! - Code inside `#[cfg(test)]` modules.
//! - `assert!` and `assert_eq!` — not flagged in this version. If a need is
//!   identified in practice, a `check_asserts` config option could be added.
//!   Excluded from this version to avoid noise in contracts that use
//!   `assert!` as a guard (some SDK examples do this); see docs/rules/SOR006.md.
//!
//! # Call-graph integration
//!
//! Helpers that are called by entrypoints are also checked. A finding is
//! emitted at the location of the panic-producing call in the helper, along
//! with the name of the enclosing helper function.
//!
//! # References
//!
//! - README: [SOR006 · panic-in-entrypoint](../../../../../README.md#sor006--panic-in-entrypoint)

use syn::visit::Visit;
use syn::{Block, Expr, ExprIndex, ExprMacro, ExprMethodCall};

use lint_core::{util::has_cfg_test, CallGraph, Finding, Rule, RuleContext, Severity};

const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR006.md";

/// Macro names that produce a panic.
const PANIC_MACROS: &[&str] = &["panic", "unreachable", "todo", "unimplemented"];

/// Method names that produce a panic.
const PANIC_METHODS: &[&str] = &["unwrap", "expect"];

/// Implementation of the `panic-in-entrypoint` rule.
#[derive(Debug, Default)]
pub struct Sor006;

impl Rule for Sor006 {
    fn id(&self) -> &'static str {
        "SOR006"
    }

    fn name(&self) -> &'static str {
        "panic-in-entrypoint"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let severity = ctx.config().severity_or(self.default_severity());

        // Determine if there is a known #[contracterror] enum to mention in help.
        let error_enum_hint = ctx
            .model()
            .error_enum_names()
            .first()
            .copied()
            .map(str::to_owned);

        let cg = CallGraph::build(ctx.ast());
        let mut findings = Vec::new();

        // Check each entrypoint and all helpers it can reach.
        for impl_block in &ctx.model().impls {
            for ep in &impl_block.entrypoints {
                scan_function(
                    ep.block,
                    &ep.name,
                    ctx,
                    severity,
                    &error_enum_hint,
                    &mut findings,
                );

                // Walk helpers reachable from this entrypoint.
                for helper in all_helpers(ctx) {
                    if cg.reaches(&ep.name, &|n| n == helper.name.as_str()) {
                        scan_function(
                            helper.block,
                            &helper.name,
                            ctx,
                            severity,
                            &error_enum_hint,
                            &mut findings,
                        );
                    }
                }
            }
        }

        // Deduplicate by (file, line, col, rule_id) — a helper may be
        // reachable from multiple entrypoints.
        findings.sort_by(|a, b| {
            (&a.file, a.line, a.column, &a.rule_id).cmp(&(&b.file, b.line, b.column, &b.rule_id))
        });
        findings.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.column == b.column);

        findings
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Returns an iterator over all helper functions (free + impl helpers).
fn all_helpers<'a>(ctx: &'a RuleContext<'_>) -> impl Iterator<Item = &'a lint_core::HelperFn<'a>> {
    ctx.model()
        .helpers
        .iter()
        .chain(ctx.model().impls.iter().flat_map(|imp| imp.helpers.iter()))
}

fn scan_function(
    block: &Block,
    fn_name: &str,
    ctx: &RuleContext<'_>,
    severity: Severity,
    error_enum_hint: &Option<String>,
    findings: &mut Vec<Finding>,
) {
    let mut visitor = Sor006Visitor {
        fn_name: fn_name.to_owned(),
        path: ctx.path(),
        severity,
        error_enum_hint,
        findings,
        in_cfg_test: false,
    };
    visitor.visit_block(block);
}

// ─── Visitor ─────────────────────────────────────────────────────────────────

struct Sor006Visitor<'a> {
    fn_name: String,
    path: &'a std::path::Path,
    severity: Severity,
    error_enum_hint: &'a Option<String>,
    findings: &'a mut Vec<Finding>,
    /// Whether we are currently inside a `#[cfg(test)]` item.
    in_cfg_test: bool,
}

impl<'a, 'ast> Visit<'ast> for Sor006Visitor<'a> {
    /// Skip any item with #[cfg(test)].
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs = match item {
            syn::Item::Fn(f) => &f.attrs,
            syn::Item::Mod(m) => &m.attrs,
            _ => {
                syn::visit::visit_item(self, item);
                return;
            }
        };
        if has_cfg_test(attrs) {
            // Don't recurse into test items.
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        if self.in_cfg_test {
            return;
        }
        let method = call.method.to_string();
        if PANIC_METHODS.contains(&method.as_str()) {
            let (line, col) = lint_core::util::span_location(call.method.span());
            self.emit(line, col, &method, PanicKind::Method);
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_macro(&mut self, mac: &'ast ExprMacro) {
        if self.in_cfg_test {
            return;
        }
        let name = mac
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if PANIC_MACROS.contains(&name.as_str()) {
            let (line, col) =
                lint_core::util::span_location(mac.mac.path.segments.last().unwrap().ident.span());
            self.emit(line, col, &name, PanicKind::Macro);
        }
        syn::visit::visit_expr_macro(self, mac);
    }

    fn visit_stmt_macro(&mut self, mac: &'ast syn::StmtMacro) {
        if self.in_cfg_test {
            return;
        }
        let name = mac
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if PANIC_MACROS.contains(&name.as_str()) {
            let (line, col) =
                lint_core::util::span_location(mac.mac.path.segments.last().unwrap().ident.span());
            self.emit(line, col, &name, PanicKind::Macro);
        }
        syn::visit::visit_stmt_macro(self, mac);
    }

    fn visit_expr_index(&mut self, index: &'ast ExprIndex) {
        if self.in_cfg_test {
            return;
        }
        // Flag indexing where the index is not a constant/literal.
        if !is_constant_index(&index.index) {
            let (line, col) = lint_core::util::span_location(index.bracket_token.span.open());
            self.emit(line, col, "[]", PanicKind::Index);
        }
        syn::visit::visit_expr_index(self, index);
    }
}

#[derive(Debug, Clone, Copy)]
enum PanicKind {
    Method,
    Macro,
    Index,
}

impl<'a> Sor006Visitor<'a> {
    fn emit(&mut self, line: usize, col: usize, pattern: &str, kind: PanicKind) {
        let (description, detail) = match kind {
            PanicKind::Method => (
                format!("`.{pattern}()` will panic"),
                format!("in function `{}`", self.fn_name),
            ),
            PanicKind::Macro => (
                format!("`{pattern}!` will panic"),
                format!("in function `{}`", self.fn_name),
            ),
            PanicKind::Index => (
                "direct index with non-constant index will panic on out-of-bounds".to_owned(),
                format!("in function `{}`", self.fn_name),
            ),
        };

        let message = format!("{description} {detail}");

        let help = if let Some(error_name) = self.error_enum_hint {
            format!(
                "use `ok_or({error_name}::...)? ` or a `match`/`if let` \
                 to propagate a typed error instead of panicking"
            )
        } else {
            "use `.ok_or(Error::...)?` or a `match`/`if let` to propagate a \
             typed `#[contracterror]` error instead of panicking"
                .to_owned()
        };

        self.findings.push(
            Finding::new(
                "SOR006",
                "panic-in-entrypoint",
                self.severity,
                message,
                self.path,
                line,
                col,
            )
            .with_help(help)
            .with_docs_url(DOCS_URL),
        );
    }
}

/// Returns `true` when the expression is a constant (literal or path to a
/// named constant). Used to avoid flagging `arr[0]` or `arr[MY_CONST]`.
fn is_constant_index(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(_) => true,
        Expr::Path(path) => {
            // A path like `MY_CONST` or `consts::MAX` is treated as constant.
            path.path.segments.iter().all(|s| {
                // Convention: ALL_CAPS identifiers are constants.
                let name = s.ident.to_string();
                name == name.to_ascii_uppercase()
            })
        }
        Expr::Unary(u) => is_constant_index(&u.expr),
        Expr::Paren(p) => is_constant_index(&p.expr),
        Expr::Cast(c) => is_constant_index(&c.expr),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::{ContractModel, RuleConfig, RuleContext};
    use syn::parse_quote;

    fn run(file: syn::File) -> Vec<Finding> {
        let path = std::path::Path::new("test.rs");
        let model = ContractModel::build(&file);
        let config = RuleConfig::default();
        let ctx = RuleContext::new(&file, path, "", &model, &config);
        Sor006.check(&ctx)
    }

    #[test]
    fn flags_unwrap() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn get_admin(env: Env) -> Address {
                    env.storage().instance().get(&DataKey::Admin).unwrap()
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("unwrap"));
    }

    #[test]
    fn flags_expect() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn get_admin(env: Env) -> Address {
                    env.storage().instance().get(&DataKey::Admin).expect("admin must exist")
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("expect"));
    }

    #[test]
    fn flags_panic_macro() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn do_thing(env: Env) {
                    panic!("not implemented");
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("panic"));
    }

    #[test]
    fn does_not_flag_unwrap_or() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn balance(env: Env, who: Address) -> i128 {
                    env.storage().persistent().get(&DataKey::Balance(who)).unwrap_or(0)
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn does_not_flag_ok_or_question_mark() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn get_admin(env: Env) -> Result<Address, u32> {
                    let admin = env.storage().instance().get(&DataKey::Admin).ok_or(1u32)?;
                    Ok(admin)
                }
            }
        };
        assert!(run(file).is_empty());
    }
}
