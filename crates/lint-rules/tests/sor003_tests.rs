//! Snapshot tests for SOR003 `missing-ttl-extension`.

mod harness;

use lint_rules::sor003::Sor003;

/// The README's vulnerable example: persistent write without extend_ttl.
#[test]
fn sor003_vulnerable_missing_ttl() {
    assert_rule_snapshot!(Sor003, "SOR003", "vulnerable", "sor003_missing_ttl.rs");
}

/// Persistent write with only an instance extend_ttl should still be flagged.
#[test]
fn sor003_vulnerable_wrong_tier_ttl() {
    assert_rule_snapshot!(Sor003, "SOR003", "vulnerable", "sor003_wrong_tier_ttl.rs");
}

/// The README's safe example: persistent write with persistent extend_ttl.
#[test]
fn sor003_safe_with_ttl() {
    assert_rule_snapshot!(Sor003, "SOR003", "safe", "sor003_with_ttl.rs");
}

/// Instance write with instance extend_ttl should be safe.
#[test]
fn sor003_safe_instance_with_ttl() {
    assert_rule_snapshot!(Sor003, "SOR003", "safe", "sor003_instance_with_ttl.rs");
}

/// extend_ttl via private helper must not be flagged (call-graph credit).
#[test]
fn sor003_safe_ttl_via_helper() {
    assert_rule_snapshot!(Sor003, "SOR003", "safe", "sor003_ttl_via_helper.rs");
}
