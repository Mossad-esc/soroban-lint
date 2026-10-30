//! SOR001 `missing-require-auth`
//!
//! Detects public entrypoints in a `#[contractimpl]` block that write to
//! contract storage but never call `require_auth` or `require_auth_for_args`
//! — either directly in the entrypoint body **or** transitively through a
//! private helper function reachable via the crate call graph.
//!
//! # What counts as a storage write?
//!
//! Any method-call chain that contains `storage()` and ends in one of the
//! following mutating methods:
//!
//! - `set`
//! - `remove`
//! - `bump` (deprecated alias)
//! - `extend_ttl`
//!
//! The call graph is also used to detect storage writes delegated to helpers.
//!
//! # What counts as an auth call?
//!
//! A call to `require_auth` or `require_auth_for_args` anywhere in the
//! entrypoint body (direct) *or* reachable through the call graph (indirect).
//!
//! # References
//!
//! - README: [SOR001 · missing-require-auth](../../../../../README.md#sor001--missing-require-auth)
//! - Soroban SDK: [`Address::require_auth`](https://docs.rs/soroban-sdk/latest/soroban_sdk/struct.Address.html)

use syn::visit::Visit;
use syn::{Block, Expr, ExprMethodCall};

use lint_core::{util, CallGraph, Entrypoint, Finding, Rule, RuleContext, Severity};

const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR001.md";

const STORAGE_MUTATING_METHODS: &[&str] = &["set", "remove", "bump", "extend_ttl"];
const AUTH_METHODS: &[&str] = &["require_auth", "require_auth_for_args"];

/// Implementation of the `missing-require-auth` rule.
#[derive(Debug, Default)]
pub struct Sor001;

impl Rule for Sor001 {
    fn id(&self) -> &'static str {
        "SOR001"
    }

    fn name(&self) -> &'static str {
        "missing-require-auth"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let severity = ctx.config().severity_or(self.default_severity());
        let allow = &ctx.config().allow_function_names;

        // Build the call graph for this file so we can credit helpers.
        let cg = CallGraph::build(ctx.ast());

        let mut findings = Vec::new();

        for impl_block in &ctx.model().impls {
            for entrypoint in &impl_block.entrypoints {
                // Skip intentionally-permissionless functions listed in config.
                if allow.iter().any(|name| name == &entrypoint.name) {
                    continue;
                }
                // `__constructor` is called exactly once by the Soroban deployment
                // framework and cannot be re-invoked by an arbitrary user.
                // Requiring auth inside a constructor is not possible (there is no
                // caller address yet), so we exempt it from this rule.
                if entrypoint.name == "__constructor" {
                    continue;
                }

                if let Some(finding) = check_entrypoint(entrypoint, ctx.path(), severity, &cg) {
                    findings.push(finding);
                }
            }
        }

        findings
    }
}

/// Checks one entrypoint using both direct body scan and call-graph lookup.
fn check_entrypoint(
    entrypoint: &Entrypoint<'_>,
    path: &std::path::Path,
    severity: Severity,
    cg: &CallGraph,
) -> Option<Finding> {
    let body = scan_block(entrypoint.block);

    // Storage write: direct in body OR via a helper reachable in the graph.
    let writes_storage = body.writes_storage || lint_core::fn_writes_storage(cg, &entrypoint.name);

    // Auth: direct in body OR via a helper reachable in the graph.
    let has_auth = body.has_auth || lint_core::fn_has_auth(cg, &entrypoint.name);

    if writes_storage && !has_auth {
        let message = format!(
            "entrypoint `{}` writes to storage but never calls `require_auth`",
            entrypoint.name
        );
        Some(
            Finding::new(
                "SOR001",
                "missing-require-auth",
                severity,
                message,
                path,
                entrypoint.line,
                entrypoint.column,
            )
            .with_help("call `from.require_auth()` before mutating state")
            .with_docs_url(DOCS_URL),
        )
    } else {
        None
    }
}

#[derive(Debug, Default)]
struct BodyInfo {
    writes_storage: bool,
    has_auth: bool,
}

fn scan_block(block: &Block) -> BodyInfo {
    let mut visitor = BodyVisitor::default();
    visitor.visit_block(block);
    BodyInfo {
        writes_storage: visitor.writes_storage,
        has_auth: visitor.has_auth,
    }
}

#[derive(Default)]
struct BodyVisitor {
    writes_storage: bool,
    has_auth: bool,
}

impl<'ast> Visit<'ast> for BodyVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let method = call.method.to_string();
        if AUTH_METHODS.contains(&method.as_str()) {
            self.has_auth = true;
        }
        if STORAGE_MUTATING_METHODS.contains(&method.as_str()) {
            let expr = Expr::MethodCall(call.clone());
            if util::chain_contains(&expr, "storage") {
                self.writes_storage = true;
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        syn::visit::visit_expr(self, expr);
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
        Sor001.check(&ctx)
    }

    #[test]
    fn flags_transfer_without_auth() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    env.storage().persistent().set(&DataKey::Balance(from), &amount);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "SOR001");
        assert!(findings[0].message.contains("transfer"));
    }

    #[test]
    fn does_not_flag_when_auth_present() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
                    from.require_auth();
                    env.storage().persistent().set(&DataKey::Balance(from), &amount);
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn credits_auth_in_private_helper() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, amount: i128) {
                    Self::assert_auth(&from);
                    env.storage().persistent().set(&DataKey::Balance(from), &amount);
                }
                fn assert_auth(who: &Address) { who.require_auth(); }
            }
        };
        assert!(
            run(file).is_empty(),
            "auth in private helper should be credited"
        );
    }

    #[test]
    fn helper_writes_storage_no_auth_is_flagged() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn entry(env: Env) {
                    Self::write_data(&env);
                }
                fn write_data(env: &Env) {
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
            }
        };
        assert_eq!(
            run(file).len(),
            1,
            "storage write in helper without any auth should be flagged"
        );
    }

    #[test]
    fn allow_function_names_skips_entrypoint() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn deposit(env: Env, to: Address, amount: i128) {
                    env.storage().persistent().set(&DataKey::Balance(to), &amount);
                }
            }
        };
        let path = std::path::Path::new("test.rs");
        let model = ContractModel::build(&file);
        let config = RuleConfig {
            allow_function_names: vec!["deposit".to_owned()],
            ..RuleConfig::default()
        };
        let ctx = RuleContext::new(&file, path, "", &model, &config);
        assert!(Sor001.check(&ctx).is_empty());
    }

    #[test]
    fn does_not_flag_readonly_entrypoint() {
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
    fn auth_in_nested_if_counts() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, amount: i128) {
                    if amount > 0 { from.require_auth(); }
                    env.storage().persistent().set(&DataKey::Balance(from), &amount);
                }
            }
        };
        assert!(run(file).is_empty());
    }
}
