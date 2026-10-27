//! SOR002 `unchecked-arithmetic`
//!
//! Detects two related problems:
//!
//! 1. **AST analysis** – raw `+`, `-`, `*`, `/`, `%`, and compound-assignment
//!    variants (`+=`, `-=`, `*=`, `/=`, `%=`) applied to integer-typed values
//!    inside public entrypoints.  Without type resolution the rule uses
//!    heuristics:
//!
//!    - Operands with integer literal suffixes (`1i128`, `0u64`, …).
//!    - Explicit type annotations on `let` bindings or function parameters
//!      (`i128`, `u128`, `i64`, `u64`, `i32`, `u32`, `i16`, `u16`, `i8`,
//!      `u8`, `usize`, `isize`).
//!    - Named variables that were bound from any of the above.
//!
//!    To keep false positives low the rule deliberately prefers false negatives
//!    and does **not** flag:
//!
//!    - Arithmetic where *all* operands appear to be non-integer.
//!    - Arithmetic inside `const`/`static` items.
//!    - Arithmetic on literals only (e.g. `1 + 1`).
//!
//! 2. **Manifest inspection** – parses the workspace-root `Cargo.toml` and
//!    emits one finding when `[profile.release]` does not set
//!    `overflow-checks = true`.  Controlled by `check_cargo_profile`
//!    (default `true`).
//!
//! # False-positive caveat
//!
//! Because the rule operates on the pre-expansion syntax tree without a type
//! resolver it can miss arithmetic on non-integer types that happen to share a
//! name, and can spuriously flag arithmetic on wrappers around integers.  If a
//! finding is a false positive, suppress it inline with a reason and open an
//! issue so the heuristic can be improved.
//!
//! # References
//!
//! - README §SOR002: `unchecked-arithmetic`
//! - docs/rules/SOR002.md

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use syn::visit::Visit;
use syn::{
    BinOp, Block, Expr, ExprBinary, ExprLit, FnArg, Lit, LitInt, Local, Pat, PatType, Stmt, Type,
};

use lint_core::{Entrypoint, Finding, Rule, RuleContext, Severity};

/// Documentation URL for this rule.
const DOCS_URL: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR002.md";

/// Per-invocation configuration for SOR002.
///
/// Populated from `[rules.SOR002]` in `soroban-lint.toml` once config loading
/// lands (Commit 3).  The struct is public so the config layer can set it.
#[derive(Debug, Clone)]
pub struct Sor002Config {
    /// Also inspect the workspace `Cargo.toml` for `overflow-checks = true`.
    pub check_cargo_profile: bool,
}

impl Default for Sor002Config {
    fn default() -> Self {
        Self {
            check_cargo_profile: true,
        }
    }
}

/// Primitive integer type names the heuristic recognises.
const INT_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
];

// ─── Operator helpers ────────────────────────────────────────────────────────

fn is_arith_op(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Add(_)
            | BinOp::Sub(_)
            | BinOp::Mul(_)
            | BinOp::Div(_)
            | BinOp::Rem(_)
            | BinOp::AddAssign(_)
            | BinOp::SubAssign(_)
            | BinOp::MulAssign(_)
            | BinOp::DivAssign(_)
            | BinOp::RemAssign(_)
    )
}

fn is_compound_assign(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::AddAssign(_)
            | BinOp::SubAssign(_)
            | BinOp::MulAssign(_)
            | BinOp::DivAssign(_)
            | BinOp::RemAssign(_)
    )
}

fn op_description(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add(_) | BinOp::AddAssign(_) => "`+`",
        BinOp::Sub(_) | BinOp::SubAssign(_) => "`-`",
        BinOp::Mul(_) | BinOp::MulAssign(_) => "`*`",
        BinOp::Div(_) | BinOp::DivAssign(_) => "`/`",
        BinOp::Rem(_) | BinOp::RemAssign(_) => "`%`",
        _ => "arithmetic",
    }
}

// ─── Integer-ness heuristics ─────────────────────────────────────────────────

fn type_path_is_int(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        return tp
            .path
            .segments
            .last()
            .is_some_and(|s| INT_TYPES.contains(&s.ident.to_string().as_str()));
    }
    false
}

fn lit_int_has_int_suffix(lit: &LitInt) -> bool {
    let suffix = lit.suffix();
    !suffix.is_empty() && INT_TYPES.contains(&suffix)
}

/// Whether `expr` "looks like" an integer value.
///
/// Returns `true` only when there is a positive signal (suffix, type
/// annotation, cast, or known variable name).  Defaults to `false` for
/// unknown/opaque expressions to keep false-positive rate low.
fn expr_looks_integer(expr: &Expr, int_names: &HashSet<String>) -> bool {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => lit_int_has_int_suffix(lit),
        Expr::Path(ep) => ep
            .path
            .get_ident()
            .is_some_and(|id| int_names.contains(&id.to_string())),
        Expr::Cast(c) => type_path_is_int(&c.ty),
        Expr::Paren(p) => expr_looks_integer(&p.expr, int_names),
        Expr::Unary(u) => expr_looks_integer(&u.expr, int_names),
        _ => false,
    }
}

/// `true` when at least one operand looks integer and not both are bare literals.
fn binary_operands_look_integer(left: &Expr, right: &Expr, int_names: &HashSet<String>) -> bool {
    let both_bare_lit = matches!(left, Expr::Lit(_)) && matches!(right, Expr::Lit(_));
    if both_bare_lit {
        return false;
    }
    expr_looks_integer(left, int_names) || expr_looks_integer(right, int_names)
}

// ─── Binding collection ──────────────────────────────────────────────────────

/// Collect parameter names that have an explicit integer type annotation.
fn collect_int_params(ep: &Entrypoint<'_>, names: &mut HashSet<String>) {
    for arg in &ep.signature.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if type_path_is_int(ty) {
                if let Pat::Ident(id) = pat.as_ref() {
                    names.insert(id.ident.to_string());
                }
            }
        }
    }
}

/// Collect `let`-binding names that have an explicit integer annotation or are
/// initialised from a known integer expression.
fn collect_int_bindings(block: &Block, names: &mut HashSet<String>) {
    for stmt in &block.stmts {
        let Stmt::Local(local) = stmt else { continue };
        let Local { pat, init, .. } = local;

        // `let x: i128 = …`
        if let Pat::Type(PatType { pat, ty, .. }) = pat {
            if type_path_is_int(ty) {
                if let Pat::Ident(id) = pat.as_ref() {
                    names.insert(id.ident.to_string());
                }
            }
        }

        // `let x = <integer expr>` — propagate.
        if let (Pat::Ident(id), Some(init)) = (pat, init) {
            if expr_looks_integer(&init.expr, names) {
                names.insert(id.ident.to_string());
            }
        }
    }
}

// ─── AST visitor ─────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Hit {
    line: usize,
    column: usize,
    op: &'static str,
}

struct ArithVisitor<'a> {
    int_names: &'a HashSet<String>,
    hits: Vec<Hit>,
}

impl<'a, 'ast> Visit<'ast> for ArithVisitor<'a> {
    // Do not descend into const / static items.
    fn visit_item_const(&mut self, _: &'ast syn::ItemConst) {}
    fn visit_item_static(&mut self, _: &'ast syn::ItemStatic) {}

    fn visit_expr_binary(&mut self, expr: &'ast ExprBinary) {
        if is_arith_op(&expr.op) {
            let should_flag = if is_compound_assign(&expr.op) {
                // `x += y` — left side is the target variable.
                let left_is_int = if let Expr::Path(ep) = expr.left.as_ref() {
                    ep.path
                        .get_ident()
                        .is_some_and(|id| self.int_names.contains(&id.to_string()))
                } else {
                    false
                };
                let right_is_int = expr_looks_integer(&expr.right, self.int_names);
                left_is_int || right_is_int
            } else {
                binary_operands_look_integer(&expr.left, &expr.right, self.int_names)
            };

            if should_flag {
                // Get the span from the expression node via ToTokens / spanned.
                // We use the left operand's span as a proxy for the operator
                // location; it is always on the same line.
                use syn::spanned::Spanned;
                let span = expr.op.span();
                let start = span.start();
                self.hits.push(Hit {
                    line: start.line,
                    column: start.column + 1,
                    op: op_description(&expr.op),
                });
            }
        }
        syn::visit::visit_expr_binary(self, expr);
    }
}

// ─── Rule ────────────────────────────────────────────────────────────────────

/// Implementation of the `unchecked-arithmetic` rule.
#[derive(Debug, Default)]
pub struct Sor002;

impl Rule for Sor002 {
    fn id(&self) -> &'static str {
        "SOR002"
    }

    fn name(&self) -> &'static str {
        "unchecked-arithmetic"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding> {
        let severity = ctx.config().severity_or(self.default_severity());
        let rule_cfg = Sor002Config::default();
        let mut findings = Vec::new();

        // ── Part 1: AST analysis ──────────────────────────────────────────
        let cg = lint_core::CallGraph::build(ctx.ast());

        for impl_block in &ctx.model().impls {
            for ep in &impl_block.entrypoints {
                let mut int_names = HashSet::new();
                collect_int_params(ep, &mut int_names);
                collect_int_bindings(ep.block, &mut int_names);

                let mut visitor = ArithVisitor {
                    int_names: &int_names,
                    hits: Vec::new(),
                };
                visitor.visit_block(ep.block);

                // Also check helpers reachable from this entrypoint via the
                // call graph: walk the file for functions called transitively.
                // (Simple: re-scan all helpers whose names are reachable.)
                for impl_b2 in &ctx.model().impls {
                    for helper in &impl_b2.helpers {
                        if cg.reaches(&ep.name, &|n| n == helper.name.as_str()) {
                            let mut h_names = HashSet::new();
                            // Seed with params of helper too.
                            for arg in &helper.signature.inputs {
                                if let syn::FnArg::Typed(syn::PatType { pat, ty, .. }) = arg {
                                    if type_path_is_int(ty) {
                                        if let syn::Pat::Ident(id) = pat.as_ref() {
                                            h_names.insert(id.ident.to_string());
                                        }
                                    }
                                }
                            }
                            h_names.extend(int_names.iter().cloned());
                            collect_int_bindings(helper.block, &mut h_names);
                            let mut hv = ArithVisitor {
                                int_names: &h_names,
                                hits: Vec::new(),
                            };
                            hv.visit_block(helper.block);
                            for hit in hv.hits {
                                let msg = format!(
                                    "entrypoint `{}` (via helper `{}`) uses unchecked arithmetic operator {} on an integer value",
                                    ep.name, helper.name, hit.op,
                                );
                                findings.push(
                                    Finding::new(
                                        "SOR002",
                                        "unchecked-arithmetic",
                                        severity,
                                        msg,
                                        ctx.path(),
                                        hit.line,
                                        hit.column,
                                    )
                                    .with_help(
                                        "use `checked_add`, `checked_sub`, `checked_mul`, etc. \
                                         and handle the overflow case, or set \
                                         `overflow-checks = true` under `[profile.release]` \
                                         in your workspace Cargo.toml",
                                    )
                                    .with_docs_url(DOCS_URL),
                                );
                            }
                        }
                    }
                }

                for hit in visitor.hits {
                    let msg = format!(
                        "entrypoint `{}` uses unchecked arithmetic operator {} on an integer value",
                        ep.name, hit.op,
                    );
                    findings.push(
                        Finding::new(
                            "SOR002",
                            "unchecked-arithmetic",
                            severity,
                            msg,
                            ctx.path(),
                            hit.line,
                            hit.column,
                        )
                        .with_help(
                            "use `checked_add`, `checked_sub`, `checked_mul`, etc. \
                             and handle the overflow case, or set \
                             `overflow-checks = true` under `[profile.release]` \
                             in your workspace Cargo.toml",
                        )
                        .with_docs_url(DOCS_URL),
                    );
                }
            }
        }

        // ── Part 2: Manifest inspection ───────────────────────────────────
        if rule_cfg.check_cargo_profile {
            if let Some(manifest) = manifest_for_file(ctx.path()) {
                if let Some(f) = check_cargo_profile(&manifest, severity) {
                    findings.push(f);
                }
            }
        }

        findings
    }
}

/// Reads `Cargo.toml` at `manifest_path` and checks `[profile.release]` for
/// `overflow-checks = true`.
///
/// Returns `None` when the manifest cannot be read/parsed, when no
/// `[profile.release]` section exists, or when the check is already enabled.
pub fn check_cargo_profile(manifest_path: &Path, severity: Severity) -> Option<Finding> {
    let source = std::fs::read_to_string(manifest_path).ok()?;
    let doc: toml::Value = toml::from_str(&source).ok()?;

    // Only report when a [profile.release] section is present.
    let release = doc.get("profile")?.get("release")?;
    let overflow_checks = release.get("overflow-checks");

    if overflow_checks.and_then(toml::Value::as_bool) == Some(true) {
        return None;
    }

    let message = if overflow_checks.is_none() {
        "`[profile.release]` does not set `overflow-checks = true`; \
         release builds will wrap on integer overflow"
            .to_owned()
    } else {
        "`[profile.release]` sets `overflow-checks` to a value other than `true`; \
         release builds may wrap on integer overflow"
            .to_owned()
    };

    Some(
        Finding::new(
            "SOR002",
            "unchecked-arithmetic",
            severity,
            message,
            manifest_path,
            1,
            1,
        )
        .with_help(
            "add `overflow-checks = true` under `[profile.release]` \
             in your workspace root `Cargo.toml`",
        )
        .with_docs_url(DOCS_URL),
    )
}

/// Walks upward from `file_path` to find the nearest `Cargo.toml`.
pub fn manifest_for_file(file_path: &Path) -> Option<PathBuf> {
    let mut dir = if file_path.is_file() {
        file_path.parent()?.to_path_buf()
    } else {
        file_path.to_path_buf()
    };
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::{ContractModel, RuleConfig, RuleContext};
    use syn::parse_quote;

    fn run(file: syn::File) -> Vec<Finding> {
        // Use a path that has no parent Cargo.toml so manifest check is skipped.
        let path = std::path::Path::new("test.rs");
        let model = ContractModel::build(&file);
        let config = RuleConfig::default();
        let ctx = RuleContext::new(&file, path, "", &model, &config);
        Sor002.check(&ctx)
    }

    #[test]
    fn flags_raw_add_on_i128_param() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn transfer(_env: Env, amount: i128) {
                    let _x = amount + 100i128;
                }
            }
        };
        let findings = run(file);
        assert!(!findings.is_empty(), "expected a finding");
        assert_eq!(findings[0].rule_id, "SOR002");
        assert!(findings[0].message.contains("transfer"));
    }

    #[test]
    fn does_not_flag_checked_add() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn deposit(_env: Env, amount: i128) {
                    let bal: i128 = 0i128;
                    let _ = bal.checked_add(amount);
                }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn does_not_flag_literal_only() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn noop(_env: Env) -> i32 { 1 + 2 }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn flags_compound_assign_on_annotated_binding() {
        let file: syn::File = parse_quote! {
            #[contractimpl]
            impl Token {
                pub fn inc(_env: Env) {
                    let mut count: i64 = 0i64;
                    count += 1i64;
                }
            }
        };
        assert!(
            !run(file).is_empty(),
            "compound assign on i64 should be flagged"
        );
    }

    #[test]
    fn does_not_flag_non_contractimpl() {
        let file: syn::File = parse_quote! {
            impl Token {
                pub fn helper(amount: i128) -> i128 { amount + 1i128 }
            }
        };
        assert!(run(file).is_empty());
    }

    #[test]
    fn profile_missing_key_returns_finding() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let m = dir.path().join("Cargo.toml");
        std::fs::write(
            &m,
            "[package]\nname=\"t\"\n[profile.release]\nopt-level=3\n",
        )
        .unwrap();
        assert!(check_cargo_profile(&m, Severity::Warning).is_some());
    }

    #[test]
    fn profile_set_to_true_returns_none() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let m = dir.path().join("Cargo.toml");
        std::fs::write(
            &m,
            "[package]\nname=\"t\"\n[profile.release]\noverflow-checks=true\n",
        )
        .unwrap();
        assert!(check_cargo_profile(&m, Severity::Warning).is_none());
    }

    #[test]
    fn profile_absent_returns_none() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let m = dir.path().join("Cargo.toml");
        std::fs::write(&m, "[package]\nname=\"t\"\n").unwrap();
        assert!(check_cargo_profile(&m, Severity::Warning).is_none());
    }
}
