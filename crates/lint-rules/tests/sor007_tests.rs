//! Snapshot tests for SOR007 `unprotected-initialize`.

mod harness;

use lint_core::{ContractModel, Rule, RuleConfig, RuleContext};
use lint_rules::sor007::Sor007;

/// The README's vulnerable example: no guard, no auth.
#[test]
fn sor007_vulnerable_unprotected_init() {
    assert_rule_snapshot!(Sor007, "SOR007", "vulnerable", "sor007_unprotected_init.rs");
}

/// Safe: the README's safe example with both guard and auth.
#[test]
fn sor007_safe_guarded_init() {
    assert_rule_snapshot!(Sor007, "SOR007", "safe", "sor007_guarded_init.rs");
}

/// Safe: auth via private helper (call-graph credit).
#[test]
fn sor007_safe_auth_via_helper() {
    assert_rule_snapshot!(Sor007, "SOR007", "safe", "sor007_auth_via_helper.rs");
}

/// Safe: guard-only (allowed, not flagged).
#[test]
fn sor007_safe_guard_only() {
    assert_rule_snapshot!(Sor007, "SOR007", "safe", "sor007_guard_only.rs");
}

/// Vulnerable: custom init_names config — "bootstrap" is treated as an initializer.
#[test]
fn sor007_vulnerable_custom_name() {
    let path = harness::fixture_path("vulnerable", "sor007_custom_name.rs");
    let source = std::fs::read_to_string(&path).expect("read fixture");
    let ast = syn::parse_file(&source).expect("parse fixture");
    let model = ContractModel::build(&ast);
    let config = RuleConfig {
        init_names: Some(vec!["bootstrap".to_owned()]),
        ..RuleConfig::default()
    };
    let ctx = RuleContext::new(&ast, &path, &source, &model, &config);
    let findings = Sor007.check(&ctx);
    assert_eq!(findings.len(), 1, "expected one finding for 'bootstrap'");
    assert!(findings[0].message.contains("bootstrap"));
}

/// __constructor is exempt from SOR007 (corpus FP fix).
#[test]
fn sor007_safe_constructor_exempt() {
    assert_rule_snapshot!(Sor007, "SOR007", "safe", "sor007_constructor_exempt.rs");
}
