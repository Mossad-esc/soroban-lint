//! SOR004 `critical-data-in-temporary`
//!
//! Detects writes to `temporary()` storage where the key expression's name
//! suggests critical state: admin addresses, balances, allowances, ownership,
//! configuration, or supply.
//!
//! # What counts as a key match?
//!
//! A key expression matches when, case-insensitively, any of the configured
//! `critical_keys` patterns is a **substring** of:
//!
//! - A plain identifier used as the key argument (`&my_admin_key`).
//! - An enum variant name in a path (`DataKey::Admin`, `DataKey::Balance(addr)`).
//! - A `symbol_short!` or `Symbol` literal string.
//!
//! Additionally, local variables in the same function body that are assigned
//! from one of the above forms are resolved (one level of assignment).
//!
//! # Default critical keys
//!
//! `admin`, `owner`, `balance`, `allowance`, `config`, `supply`
//!
//! The list can be replaced (not extended) via `[rules.SOR004] critical_keys`
//! in `soroban-lint.toml`.
//!
//! # References
//!
//! - README: [SOR004 · critical-data-in-temporary](../../../../../README.md#sor004--critical-data-in-temporary-storage)

use std::collections::HashMap;

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, Local, Pat, Stmt};

use lint_core::{Finding, Rule, RuleContext, Severity};

const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR004.md";

/// Default list of critical key name patterns (case-insensitive substrings).
const DEFAULT_CRITICAL_KEYS: &[&str] =
    &["admin", "owner", "balance", "allowance", "config", "supply"];

/// Implementation of the `critical-data-in-temporary` rule.
#[derive(Debug, Default)]
pub struct Sor004;

impl Rule for Sor004 {
    fn id(&self) -> &'static str {
        "SOR004"
    }

    fn name(&self) -> &'static str {
        "critical-data-in-temporary"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let severity = ctx.config().severity_or(self.default_severity());
        let critical_keys: Vec<String> = match &ctx.config().critical_keys {
            Some(keys) => keys.iter().map(|k| k.to_ascii_lowercase()).collect(),
            None => DEFAULT_CRITICAL_KEYS
                .iter()
                .map(|k| k.to_ascii_lowercase())
                .collect(),
        };

        let mut findings = Vec::new();

        for impl_block in &ctx.model().impls {
            for ep in &impl_block.entrypoints {
                let mut visitor = Sor004Visitor::new(&critical_keys);
                visitor.visit_block(ep.block);
                for (line, column, key_name) in visitor.violations {
                    let message = format!(
                        "critical key `{key_name}` is written to `temporary()` storage; \
                         critical state should use `instance()` or `persistent()` storage"
                    );
                    findings.push(
                        Finding::new(
                            "SOR004",
                            "critical-data-in-temporary",
                            severity,
                            message,
                            ctx.path(),
                            line,
                            column,
                        )
                        .with_help(
                            "use `env.storage().instance()` for contract-wide critical state \
                             or `env.storage().persistent()` for per-user critical state; \
                             `temporary()` storage can silently expire",
                        )
                        .with_docs_url(DOCS_URL),
                    );
                }
            }
        }

        findings
    }
}

// ─── Visitor ─────────────────────────────────────────────────────────────────

/// Collects violations: (line, column, matched_key_name) triples.
struct Sor004Visitor<'a> {
    critical_keys: &'a [String],
    /// local variable name → canonical key name (the matched critical key).
    /// Populated when we see `let var = DataKey::Admin` or similar.
    local_key_map: HashMap<String, String>,
    /// Found violations: (line, column, key_name).
    violations: Vec<(usize, usize, String)>,
}

impl<'a> Sor004Visitor<'a> {
    fn new(critical_keys: &'a [String]) -> Self {
        Self {
            critical_keys,
            local_key_map: HashMap::new(),
            violations: Vec::new(),
        }
    }

    /// Returns the critical key substring matched by `text` (lowercase), if any.
    fn matches_critical(&self, text: &str) -> Option<String> {
        let lower = text.to_ascii_lowercase();
        self.critical_keys
            .iter()
            .find(|k| lower.contains(k.as_str()))
            .cloned()
    }

    /// Extracts a descriptive name from a key expression for matching.
    ///
    /// Handles:
    /// - `DataKey::Admin`                  → "Admin"
    /// - `DataKey::Balance(addr)`           → "Balance"
    /// - plain identifier `admin_key`      → "admin_key"
    /// - `symbol_short!("ADMIN")`          → "ADMIN" (via token text heuristic)
    fn key_name_from_expr(&self, expr: &Expr) -> Option<String> {
        // Strip leading & or * references.
        let inner = strip_ref(expr);
        match inner {
            Expr::Path(path) => {
                // Last segment, e.g. Admin from DataKey::Admin.
                path.path.segments.last().map(|seg| seg.ident.to_string())
            }
            Expr::Call(call) => {
                // DataKey::Balance(addr) — the function is a path.
                if let Expr::Path(path) = call.func.as_ref() {
                    path.path.segments.last().map(|seg| seg.ident.to_string())
                } else {
                    None
                }
            }
            Expr::Macro(mac) => {
                // symbol_short!("ADMIN") or Symbol::short("ADMIN")
                let mac_name = mac
                    .mac
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_default();
                if mac_name.contains("symbol") || mac_name.contains("Symbol") {
                    // Extract the string literal from the macro tokens.
                    let tokens = mac.mac.tokens.to_string();
                    // Look for a quoted string like "ADMIN".
                    extract_string_literal(&tokens)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Strips outer reference expressions (`&`, `&mut`, `*`).
fn strip_ref(expr: &Expr) -> &Expr {
    match expr {
        Expr::Reference(r) => strip_ref(&r.expr),
        Expr::Unary(u) => match u.op {
            syn::UnOp::Deref(_) => strip_ref(&u.expr),
            _ => expr,
        },
        Expr::Paren(p) => strip_ref(&p.expr),
        Expr::Group(g) => strip_ref(&g.expr),
        _ => expr,
    }
}

/// Attempts to extract a string literal value from a macro token stream string.
/// E.g. `"ADMIN"` → `Some("ADMIN".to_owned())`.
fn extract_string_literal(tokens: &str) -> Option<String> {
    let trimmed = tokens.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        Some(trimmed[1..trimmed.len() - 1].to_owned())
    } else {
        None
    }
}

impl<'ast, 'a> Visit<'ast> for Sor004Visitor<'a> {
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Collect local variable assignments so we can resolve them later
        // when they appear as key arguments.
        if let Stmt::Local(Local {
            pat,
            init: Some(local_init),
            ..
        }) = stmt
        {
            if let (Pat::Ident(ident), Some(key_name)) =
                (pat, self.key_name_from_expr(&local_init.expr))
            {
                let var_name = ident.ident.to_string();
                if let Some(matched) = self.matches_critical(&key_name) {
                    self.local_key_map.insert(var_name, matched);
                }
            }
        }
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        // We are looking for `.set(key, value)` on a `.temporary()` chain.
        if call.method == "set" {
            let expr = Expr::MethodCall(call.clone());
            if is_temporary_chain(&expr) {
                // The first argument to set() is the key.
                if let Some(key_arg) = call.args.first() {
                    if let Some(matched) = self.check_key_arg(key_arg) {
                        let (line, col) = span_of_call(call);
                        self.violations.push((line, col, matched));
                    }
                }
            }
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

impl<'a> Sor004Visitor<'a> {
    /// Checks whether a key argument (to `.set()`) matches a critical key.
    /// Resolves plain identifiers as locals if needed.
    fn check_key_arg(&self, key_arg: &Expr) -> Option<String> {
        // Try direct match from the expression.
        if let Some(name) = self.key_name_from_expr(key_arg) {
            if self.matches_critical(&name).is_some() {
                return Some(name);
            }
        }
        // Try resolving as a local variable.
        let inner = strip_ref(key_arg);
        if let Expr::Path(path) = inner {
            if let Some(seg) = path.path.get_ident() {
                let var_name = seg.to_string();
                if let Some(critical) = self.local_key_map.get(&var_name) {
                    return Some(critical.clone());
                }
            }
        }
        None
    }
}

/// Returns true when the method-call chain contains `.temporary()`.
fn is_temporary_chain(expr: &Expr) -> bool {
    let mut current = expr;
    loop {
        match current {
            Expr::MethodCall(mc) => {
                if mc.method == "temporary" {
                    return true;
                }
                current = &mc.receiver;
            }
            Expr::Field(f) => current = &f.base,
            Expr::Try(t) => current = &t.expr,
            Expr::Paren(p) => current = &p.expr,
            Expr::Group(g) => current = &g.expr,
            Expr::Reference(r) => current = &r.expr,
            _ => return false,
        }
    }
}

/// Returns the 1-based line and column for a method call site (the method
/// identifier).
fn span_of_call(call: &ExprMethodCall) -> (usize, usize) {
    use lint_core::util::span_location;
    span_location(call.method.span())
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
        Sor004.check(&ctx)
    }

    fn run_with_keys(file: syn::File, keys: Vec<String>) -> Vec<Finding> {
        let path = std::path::Path::new("test.rs");
        let model = ContractModel::build(&file);
        let config = RuleConfig {
            critical_keys: Some(keys),
            ..RuleConfig::default()
        };
        let ctx = RuleContext::new(&file, path, "", &model, &config);
        Sor004.check(&ctx)
    }

    #[test]
    fn flags_admin_in_temporary() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn setup(env: Env, admin: Address) {
                    env.storage().temporary().set(&DataKey::Admin, &admin);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1, "expected one finding");
        assert_eq!(findings[0].rule_id, "SOR004");
        assert!(findings[0].message.contains("Admin"));
    }

    #[test]
    fn flags_balance_variant_with_args() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn set_bal(env: Env, addr: Address, amount: i128) {
                    env.storage().temporary().set(&DataKey::Balance(addr), &amount);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Balance"));
    }

    #[test]
    fn does_not_flag_nonce_in_temporary() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn use_nonce(env: Env, user: Address) {
                    env.storage().temporary().set(&DataKey::Nonce(user), &true);
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn does_not_flag_admin_in_instance() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn initialize(env: Env, admin: Address) {
                    env.storage().instance().set(&DataKey::Admin, &admin);
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn resolves_local_variable() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn store(env: Env, admin: Address) {
                    let key = DataKey::Admin;
                    env.storage().temporary().set(&key, &admin);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn custom_critical_keys_replace_defaults() {
        // "secret" is in the custom list; "admin" is not.
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn store(env: Env, val: u32) {
                    env.storage().temporary().set(&DataKey::Secret, &val);
                    env.storage().temporary().set(&DataKey::Admin, &val);
                }
            }
        };
        let findings = run_with_keys(file, vec!["secret".to_owned()]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Secret"));
    }
}
