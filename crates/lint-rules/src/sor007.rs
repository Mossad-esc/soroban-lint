//! SOR007 `unprotected-initialize`
//!
//! Detects initializer entrypoints that write to storage without BOTH:
//! (a) an already-initialized guard (`has()` or `get()` on a key that is
//!     subsequently set, leading to an early return or error), AND
//! (b) a `require_auth` call (directly or via the call graph).
//!
//! The rule flags an initializer when NEITHER guard is present. Having only
//! one of the two is allowed but discouraged — see docs/rules/SOR007.md for
//! the rationale.
//!
//! # Identifying initializers
//!
//! A function is treated as an initializer when its name (case-insensitive,
//! exact match) appears in the configured `init_names` list:
//! default: `initialize`, `init`, `setup`.
//!
//! The special name `__constructor` is always checked regardless of config.
//!
//! # References
//!
//! - README: [SOR007 · unprotected-initialize](../../../../../README.md#sor007--unprotected-initialize)

use syn::visit::Visit;
use syn::{Block, Expr, ExprMethodCall};

use lint_core::{CallGraph, Finding, Rule, RuleContext, Severity};

const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR007.md";

const DEFAULT_INIT_NAMES: &[&str] = &["initialize", "init", "setup"];

/// Implementation of the `unprotected-initialize` rule.
#[derive(Debug, Default)]
pub struct Sor007;

impl Rule for Sor007 {
    fn id(&self) -> &'static str {
        "SOR007"
    }

    fn name(&self) -> &'static str {
        "unprotected-initialize"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let severity = ctx.config().severity_or(self.default_severity());

        // Build the effective list of initializer names.
        let init_names: Vec<String> = match &ctx.config().init_names {
            Some(names) => names.iter().map(|n| n.to_ascii_lowercase()).collect(),
            None => DEFAULT_INIT_NAMES
                .iter()
                .map(|n| n.to_ascii_lowercase())
                .collect(),
        };

        let cg = CallGraph::build(ctx.ast());
        let mut findings = Vec::new();

        for impl_block in &ctx.model().impls {
            for ep in &impl_block.entrypoints {
                if !is_initializer_name(&ep.name, &init_names) {
                    continue;
                }

                // `__constructor` is called exactly once by the Soroban
                // deployment framework.  It cannot be called by an arbitrary
                // user, so the "can be called twice" concern does not apply.
                // Exempting it avoids the false positive confirmed in corpus
                // triage (15 occurrences in soroban-examples).
                if ep.name == "__constructor" {
                    continue;
                }

                let info = scan_block(ep.block);

                // Storage write check (direct + via call graph).
                let writes = info.writes_storage || lint_core::fn_writes_storage(&cg, &ep.name);
                if !writes {
                    // No storage write — not an init that needs protection.
                    continue;
                }

                // Auth check (direct + via call graph).
                let has_auth = info.has_auth || lint_core::fn_has_auth(&cg, &ep.name);

                // Guard check (direct in body).
                let has_guard = info.has_init_guard;

                // Flag only when NEITHER auth NOR guard is present.
                if !has_auth && !has_guard {
                    let message = format!(
                        "initializer `{}` writes storage but has neither \
                         an already-initialized guard nor a `require_auth` call; \
                         it can be called by anyone or called twice",
                        ep.name
                    );
                    findings.push(
                        Finding::new(
                            "SOR007",
                            "unprotected-initialize",
                            severity,
                            message,
                            ctx.path(),
                            ep.line,
                            ep.column,
                        )
                        .with_help(
                            "add `if env.storage().instance().has(&DataKey::Admin) { \
                             return Err(Error::AlreadyInitialized); }` and/or \
                             `admin.require_auth()` at the start of the function",
                        )
                        .with_docs_url(DOCS_URL),
                    );
                }
            }
        }

        findings
    }
}

// ─── Initializer name check ───────────────────────────────────────────────────

/// Returns true when `fn_name` is treated as an initializer.
fn is_initializer_name(fn_name: &str, init_names: &[String]) -> bool {
    let lower = fn_name.to_ascii_lowercase();
    if lower == "__constructor" {
        return true;
    }
    init_names.iter().any(|name| name == &lower)
}

// ─── Body scanner ─────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct InitInfo {
    writes_storage: bool,
    has_auth: bool,
    /// True when we see a `.has(key)` or `.get(key)` check on a key that is
    /// also set in the function, with the check leading to an early exit
    /// (return or `?`).
    has_init_guard: bool,
}

fn scan_block(block: &Block) -> InitInfo {
    let mut v = InitVisitor::default();
    v.visit_block(block);
    InitInfo {
        writes_storage: v.writes_storage,
        has_auth: v.has_auth,
        has_init_guard: v.has_guard_check && v.has_early_exit,
    }
}

#[derive(Default)]
struct InitVisitor {
    writes_storage: bool,
    has_auth: bool,
    /// Saw a `.has(...)` or `.get(...)` call on storage.
    has_guard_check: bool,
    /// Saw a `return ...` statement or an `Err(...)` or `?` expression.
    has_early_exit: bool,
}

impl<'ast> Visit<'ast> for InitVisitor {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let method = call.method.to_string();

        match method.as_str() {
            "require_auth" | "require_auth_for_args" => {
                self.has_auth = true;
            }
            "set" | "remove" => {
                // Must be on a storage chain.
                let expr = Expr::MethodCall(call.clone());
                if lint_core::util::chain_contains(&expr, "storage") {
                    self.writes_storage = true;
                }
            }
            "has" | "get" => {
                // A `.has()` or `.get()` on a storage chain is a candidate guard.
                let expr = Expr::MethodCall(call.clone());
                if lint_core::util::chain_contains(&expr, "storage") {
                    self.has_guard_check = true;
                }
            }
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_return(&mut self, _r: &'ast syn::ExprReturn) {
        self.has_early_exit = true;
        // Do not recurse — we only need to note that an early return exists.
    }

    fn visit_expr_try(&mut self, t: &'ast syn::ExprTry) {
        // The `?` operator is an early exit path.
        self.has_early_exit = true;
        syn::visit::visit_expr_try(self, t);
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
        Sor007.check(&ctx)
    }

    fn run_with_names(file: syn::File, names: Vec<String>) -> Vec<Finding> {
        let path = std::path::Path::new("test.rs");
        let model = ContractModel::build(&file);
        let config = RuleConfig {
            init_names: Some(names),
            ..RuleConfig::default()
        };
        let ctx = RuleContext::new(&file, path, "", &model, &config);
        Sor007.check(&ctx)
    }

    #[test]
    fn flags_unprotected_initialize() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) {
                    env.storage().instance().set(&DataKey::Admin, &admin);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("initialize"));
    }

    #[test]
    fn does_not_flag_with_guard_and_auth() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) -> Result<(), u32> {
                    if env.storage().instance().has(&DataKey::Admin) {
                        return Err(1u32);
                    }
                    admin.require_auth();
                    env.storage().instance().set(&DataKey::Admin, &admin);
                    Ok(())
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn does_not_flag_with_guard_only() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn initialize(env: Env, admin: Address) -> Result<(), u32> {
                    if env.storage().instance().has(&DataKey::Admin) {
                        return Err(1u32);
                    }
                    env.storage().instance().set(&DataKey::Admin, &admin);
                    Ok(())
                }
            }
        };
        // Guard alone: allowed (not flagged).
        assert!(run(file).is_empty());
    }

    #[test]
    fn does_not_flag_non_initializer() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(env: Env, from: Address) {
                    env.storage().persistent().set(&DataKey::Balance(from), &0i128);
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn custom_init_name_is_flagged() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn bootstrap(env: Env, admin: Address) {
                    env.storage().instance().set(&DataKey::Admin, &admin);
                }
            }
        };
        let findings = run_with_names(file, vec!["bootstrap".to_owned()]);
        assert_eq!(findings.len(), 1);
    }
}
