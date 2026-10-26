//! SOR001 `missing-require-auth`
//!
//! Detects public entrypoints in a `#[contractimpl]` block that write to
//! contract storage but never call `require_auth` or `require_auth_for_args`.
//!
//! # What counts as a storage write?
//!
//! Any method-call chain that contains `storage()` and ends in one of the
//! following mutating methods from the Soroban SDK storage API:
//!
//! - `set`
//! - `remove`
//! - `bump` (deprecated alias retained for backward compatibility)
//! - `extend_ttl` (TTL extension is a mutation of the entry's metadata)
//!
//! # What counts as an auth call?
//!
//! A call to `require_auth` or `require_auth_for_args` anywhere in the
//! entrypoint body, including inside nested blocks, `if` expressions, closures,
//! and `match` arms.
//!
//! # Known limitation
//!
//! Auth or storage mutations performed inside a *private helper function* are
//! not yet credited to the calling entrypoint.  This will be resolved in the
//! Day 4 call-graph pass.  Until then, contracts that delegate both storage
//! writes and auth checks to a private helper will produce a false positive.
//!
//! # References
//!
//! - README: [SOR001 · missing-require-auth](../../../../../README.md#sor001--missing-require-auth)
//! - Soroban SDK: [`Address::require_auth`](https://docs.rs/soroban-sdk/latest/soroban_sdk/struct.Address.html#method.require_auth)

use proc_macro2::Span;
use syn::visit::Visit;
use syn::{Block, Expr, ExprMethodCall};

use lint_core::{util, Entrypoint, Finding, Rule, RuleContext, Severity};

/// URL prefix for rule documentation.
const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR001.md";

/// Mutating method names on the Soroban storage API.
///
/// A method-call chain is considered a storage write when it contains
/// `storage()` anywhere in the receiver chain **and** the outermost method name
/// is one of these.
const STORAGE_MUTATING_METHODS: &[&str] = &[
    "set",
    "remove",
    "bump",       // deprecated but still in SDK
    "extend_ttl", // mutates the entry's TTL metadata
];

/// Method names that count as an authorization call.
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
        let mut findings = Vec::new();

        for impl_block in &ctx.model().impls {
            for entrypoint in &impl_block.entrypoints {
                if let Some(finding) = check_entrypoint(entrypoint, ctx.path(), severity) {
                    findings.push(finding);
                }
            }
        }

        findings
    }
}

/// Checks a single entrypoint and returns a Finding if it writes storage
/// without any auth call.
fn check_entrypoint(
    entrypoint: &Entrypoint<'_>,
    path: &std::path::Path,
    severity: Severity,
) -> Option<Finding> {
    let body_info = scan_block(entrypoint.block);

    if body_info.writes_storage && !body_info.has_auth {
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

/// Summary of what was found in a block.
#[derive(Debug, Default)]
struct BodyInfo {
    writes_storage: bool,
    has_auth: bool,
}

/// Scans an entire function body for storage writes and auth calls.
fn scan_block(block: &Block) -> BodyInfo {
    let mut visitor = BodyVisitor::default();
    visitor.visit_block(block);
    BodyInfo {
        writes_storage: visitor.writes_storage,
        has_auth: visitor.has_auth,
    }
}

/// AST visitor that walks the full body of an entrypoint.
#[derive(Default)]
struct BodyVisitor {
    writes_storage: bool,
    has_auth: bool,
}

impl<'ast> Visit<'ast> for BodyVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let method = call.method.to_string();

        // Check for an auth call at any depth in the tree.
        if AUTH_METHODS.contains(&method.as_str()) {
            self.has_auth = true;
        }

        // Check for a storage-mutating call: the outermost method must be a
        // mutating method AND the receiver chain must contain `storage()`.
        if STORAGE_MUTATING_METHODS.contains(&method.as_str()) {
            // Walk the receiver to see if `storage` appears anywhere in the
            // chain.  We treat the full expression `call` as the node so
            // chain_contains can see the whole chain.
            let expr = Expr::MethodCall(call.clone());
            if util::chain_contains(&expr, "storage") {
                self.writes_storage = true;
            }
        }

        // Always recurse so nested calls (closures, blocks, etc.) are visited.
        syn::visit::visit_expr_method_call(self, call);
    }

    /// Ensure we descend into all expression forms so we don't miss calls
    /// inside `if let`, `match`, closures, or `?`-expressions.
    fn visit_expr(&mut self, expr: &'ast Expr) {
        syn::visit::visit_expr(self, expr);
    }
}

/// Returns the span of the entrypoint identifier (for use in tests / helpers).
#[allow(dead_code)]
fn ident_span(entrypoint: &Entrypoint<'_>) -> Span {
    entrypoint.signature.ident.span()
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
        let findings = run(file);
        assert!(findings.is_empty());
    }

    #[test]
    fn does_not_flag_require_auth_for_args() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn approve(env: Env, from: Address, amount: i128) {
                    from.require_auth_for_args((amount,).into_val(&env));
                    env.storage().persistent().set(&DataKey::Allowance(from), &amount);
                }
            }
        };
        let findings = run(file);
        assert!(findings.is_empty());
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
        let findings = run(file);
        assert!(findings.is_empty());
    }

    #[test]
    fn flags_remove_without_auth() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn revoke(env: Env, user: Address) {
                    env.storage().persistent().remove(&DataKey::User(user));
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("revoke"));
    }

    #[test]
    fn flags_multiple_entrypoints_without_auth() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn write_a(env: Env) {
                    env.storage().instance().set(&DataKey::A, &1u32);
                }
                pub fn write_b(env: Env) {
                    env.storage().temporary().set(&DataKey::B, &2u32);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn finding_has_help_and_docs_url() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env) {
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].help.is_some());
        assert!(findings[0].docs_url.is_some());
        assert!(findings[0].docs_url.as_deref().unwrap().contains("SOR001"));
    }

    #[test]
    fn non_contractimpl_is_not_checked() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn transfer(env: Env) {
                    env.storage().persistent().set(&DataKey::X, &1u32);
                }
            }
        };
        let findings = run(file);
        assert!(findings.is_empty(), "non-contractimpl must not be flagged");
    }

    #[test]
    fn auth_in_nested_if_block_counts() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address, amount: i128) {
                    if amount > 0 {
                        from.require_auth();
                    }
                    env.storage().persistent().set(&DataKey::Balance(from), &amount);
                }
            }
        };
        let findings = run(file);
        assert!(findings.is_empty(), "auth inside an if block should count");
    }

    #[test]
    fn storage_set_in_closure_is_detected() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn run(env: Env) {
                    let store = || {
                        env.storage().persistent().set(&DataKey::X, &1u32);
                    };
                    store();
                }
            }
        };
        let findings = run(file);
        // A closure-based storage write without auth should still be flagged.
        assert_eq!(findings.len(), 1);
    }
}
