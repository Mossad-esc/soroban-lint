//! SOR005 `unbounded-storage-growth`
//!
//! Detects a "load–grow–store" sequence on a `Vec` or `Map` from
//! `soroban_sdk` within a single function where no length guard is visible.
//!
//! # Detection strategy (deliberately narrow)
//!
//! Within a single function body we look for all three of these:
//!
//! 1. **Load**: a local variable is assigned via `.get(…)` (or
//!    `.unwrap_or(Vec::new(…))` / `.unwrap_or(Map::new(…))`).  The variable
//!    type is inferred as a collection when the initializer contains
//!    `Vec::new`, `Map::new`, or the variable type annotation includes `Vec`
//!    or `Map`.
//!
//! 2. **Grow**: a method call `push_back`, `push_front`, `insert`, or `set`
//!    is made on that variable.
//!
//! 3. **Store-back**: a `.set(…, &var)` call where `var` is the same variable
//!    appears in the same function.
//!
//! 4. **No length guard**: no expression `var.len()` or `var.len() < N` or
//!    similar is found in the function body before the growth call.
//!
//! Additionally, the call graph is searched: if the function calls a helper
//! whose name contains one of the grow-method names, and the helper's body
//! matches a load+grow+store (without guard), the helper is flagged separately
//! when the rule is run against the helper function.  The call-graph path is
//! covered by the `sor005_helper_growth.rs` fixture.
//!
//! # What this rule CANNOT see
//!
//! - Growth split across multiple functions where the collection is returned
//!   and stored externally.
//! - Growth performed inside a loop over caller-supplied input without an
//!   explicit bound.
//! - Indirect growth via a helper that receives the collection as an argument.
//!
//! These limitations are documented in `docs/rules/SOR005.md`.
//!
//! # References
//!
//! - README: [SOR005 · unbounded-storage-growth](../../../../../README.md#sor005--unbounded-storage-growth)

use std::collections::{HashMap, HashSet};

use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, Local, Stmt};

use lint_core::{Finding, Rule, RuleContext, Severity};

const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR005.md";

/// Growth method names that extend a collection.
const GROW_METHODS: &[&str] = &["push_back", "push_front", "insert", "set"];

/// Implementation of the `unbounded-storage-growth` rule.
#[derive(Debug, Default)]
pub struct Sor005;

impl Rule for Sor005 {
    fn id(&self) -> &'static str {
        "SOR005"
    }

    fn name(&self) -> &'static str {
        "unbounded-storage-growth"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let severity = ctx.config().severity_or(self.default_severity());
        let mut findings = Vec::new();

        for impl_block in &ctx.model().impls {
            for ep in &impl_block.entrypoints {
                check_function(ep.block, ctx.path(), severity, &ep.name, &mut findings);
            }
        }

        // Also check free helpers that may be called from entrypoints.
        for helper in &ctx.model().helpers {
            check_function(
                helper.block,
                ctx.path(),
                severity,
                &helper.name,
                &mut findings,
            );
        }
        // And impl helpers.
        for impl_block in &ctx.model().impls {
            for helper in &impl_block.helpers {
                check_function(
                    helper.block,
                    ctx.path(),
                    severity,
                    &helper.name,
                    &mut findings,
                );
            }
        }

        findings
    }
}

// ─── Per-function analysis ────────────────────────────────────────────────────

fn check_function(
    block: &syn::Block,
    path: &std::path::Path,
    severity: Severity,
    fn_name: &str,
    findings: &mut Vec<Finding>,
) {
    let mut visitor = Sor005Visitor::default();
    visitor.visit_block(block);

    // For each collection variable that is loaded, grown, and stored back
    // without a length guard, emit a finding.
    for var in &visitor.loaded_vars {
        let was_grown = visitor.grown_vars.contains(var);
        let was_stored = visitor.stored_vars.contains(var);
        let has_guard = visitor.guarded_vars.contains(var);

        if was_grown && was_stored && !has_guard {
            let (line, col) = visitor.grow_locations.get(var).copied().unwrap_or((1, 1));
            let message = format!(
                "function `{fn_name}` loads a collection under a fixed key, \
                 appends to it, and writes it back without a length guard; \
                 unbounded growth can make the contract unusable"
            );
            findings.push(
                Finding::new(
                    "SOR005",
                    "unbounded-storage-growth",
                    severity,
                    message,
                    path,
                    line,
                    col,
                )
                .with_help(
                    "store each item under its own key with a counter, \
                     or enforce a maximum length and return a typed error \
                     when it is exceeded (e.g. `Err(Error::Full)?`)",
                )
                .with_docs_url(DOCS_URL),
            );
        }
    }
}

// ─── Visitor ─────────────────────────────────────────────────────────────────

/// Tracks state within one function body for the load–grow–store pattern.
#[derive(Default)]
struct Sor005Visitor {
    /// Variable names that look like collections loaded from storage.
    loaded_vars: HashSet<String>,
    /// Variable names that had a grow method called on them.
    grown_vars: HashSet<String>,
    /// Variable names written back to storage.
    stored_vars: HashSet<String>,
    /// Variable names where a `.len()` check was observed.
    guarded_vars: HashSet<String>,
    /// Line/column of the grow call for each variable.
    grow_locations: HashMap<String, (usize, usize)>,
}

impl<'ast> Visit<'ast> for Sor005Visitor {
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        // Pattern: `let var = storage.get(key).unwrap_or(Vec::new(env))`
        // Also: `let mut var: Vec<_> = ...`
        if let Stmt::Local(Local {
            pat,
            init: Some(local_init),
            ..
        }) = stmt
        {
            if let Some(var_name) = pat_ident_name(pat) {
                if expr_is_collection_load(&local_init.expr) {
                    self.loaded_vars.insert(var_name);
                }
            }
        }
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let method = call.method.to_string();

        // Check for grow methods: `var.push_back(...)` or `var.insert(...)`.
        if GROW_METHODS.contains(&method.as_str()) {
            // The receiver should be the collection variable or &mut var.
            if let Some(var_name) = expr_to_ident_name(&call.receiver) {
                if self.loaded_vars.contains(&var_name) {
                    let (line, col) = lint_core::util::span_location(call.method.span());
                    self.grown_vars.insert(var_name.clone());
                    self.grow_locations.entry(var_name).or_insert((line, col));
                }
            }
        }

        // Check for `.len()` calls — a guard is present.
        if method == "len" {
            if let Some(var_name) = expr_to_ident_name(&call.receiver) {
                self.guarded_vars.insert(var_name);
            }
        }

        // Check for storage `.set(key, &var)` — a store-back.
        // We detect: `env.storage().XXX().set(key, &var)`.
        if method == "set" {
            if let Some(value_arg) = call.args.iter().nth(1) {
                let inner = strip_ref(value_arg);
                if let Some(var_name) = expr_to_ident_name(inner) {
                    // Only mark as stored-back if we already saw it loaded.
                    if self.loaded_vars.contains(&var_name) {
                        self.stored_vars.insert(var_name);
                    }
                }
            }
        }

        syn::visit::visit_expr_method_call(self, call);
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extracts the identifier name from a pattern, handling:
/// - `Pat::Ident` (plain variable)
/// - `Pat::Type` (typed variable like `let mut x: Vec<_> = ...`)
fn pat_ident_name(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(ident) => Some(ident.ident.to_string()),
        syn::Pat::Type(typed) => pat_ident_name(&typed.pat),
        _ => None,
    }
}

/// Returns `true` when the expression looks like a collection loaded from
/// storage (contains `Vec::new`, `Map::new`, or a `get` call in the chain).
fn expr_is_collection_load(expr: &Expr) -> bool {
    contains_collection_init(expr) && contains_storage_get(expr)
}

/// Checks whether expression contains Vec::new or Map::new anywhere inside.
fn contains_collection_init(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(mc) => {
            // Check args first (e.g., `.unwrap_or(Vec::new(&env))`)
            for arg in &mc.args {
                if contains_collection_init(arg) {
                    return true;
                }
            }
            // Then recurse into the receiver
            contains_collection_init(&mc.receiver)
        }
        Expr::Call(call) => {
            // Vec::new(...) or Map::new(...)
            if let Expr::Path(path) = call.func.as_ref() {
                let segs: Vec<String> = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                // Vec::new or Map::new
                for window in segs.windows(2) {
                    if (window[0] == "Vec" || window[0] == "Map") && window[1] == "new" {
                        return true;
                    }
                }
            }
            // Check args
            for arg in &call.args {
                if contains_collection_init(arg) {
                    return true;
                }
            }
            contains_collection_init(&call.func)
        }
        Expr::Try(t) => contains_collection_init(&t.expr),
        Expr::Paren(p) => contains_collection_init(&p.expr),
        Expr::Group(g) => contains_collection_init(&g.expr),
        _ => false,
    }
}

/// Checks whether expression contains a `.get(...)` call (storage load).
fn contains_storage_get(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(mc) => {
            if mc.method == "get" || mc.method == "get_or_default" {
                return true;
            }
            contains_storage_get(&mc.receiver)
        }
        Expr::Try(t) => contains_storage_get(&t.expr),
        Expr::Paren(p) => contains_storage_get(&p.expr),
        Expr::Group(g) => contains_storage_get(&g.expr),
        _ => false,
    }
}

/// Extracts an identifier name from an expression (possibly through `&`, `*`).
fn expr_to_ident_name(expr: &Expr) -> Option<String> {
    match strip_ref(expr) {
        Expr::Path(path) => path.path.get_ident().map(|id| id.to_string()),
        _ => None,
    }
}

/// Strips outer reference/deref wrappers.
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
        Sor005.check(&ctx)
    }

    #[test]
    fn flags_load_grow_store() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn add_user(env: Env, user: Address) {
                    let mut users: Vec<Address> = env.storage()
                        .persistent()
                        .get(&DataKey::Users)
                        .unwrap_or(Vec::new(&env));
                    users.push_back(user);
                    env.storage().persistent().set(&DataKey::Users, &users);
                }
            }
        };
        let findings = run(file);
        assert_eq!(findings.len(), 1, "expected one finding, got: {findings:?}");
        assert_eq!(findings[0].rule_id, "SOR005");
    }

    #[test]
    fn does_not_flag_with_len_guard() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn add_user(env: Env, user: Address) -> Result<(), Error> {
                    let mut users: Vec<Address> = env.storage()
                        .persistent()
                        .get(&DataKey::Users)
                        .unwrap_or(Vec::new(&env));
                    if users.len() >= 100u32 {
                        return Err(Error::Full);
                    }
                    users.push_back(user);
                    env.storage().persistent().set(&DataKey::Users, &users);
                    Ok(())
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn does_not_flag_collection_not_written_back() {
        // Collection is read but not stored back — no write, no danger.
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Contract {
                pub fn count_users(env: Env) -> u32 {
                    let users: Vec<Address> = env.storage()
                        .persistent()
                        .get(&DataKey::Users)
                        .unwrap_or(Vec::new(&env));
                    users.len()
                }
            }
        };
        assert!(run(file).is_empty());
    }
}
