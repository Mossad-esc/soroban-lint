//! Snapshot tests for SOR002 `unchecked-arithmetic`.

mod harness;

use lint_core::Severity;
use lint_rules::sor002::{check_cargo_profile, Sor002};

/// Raw arithmetic on i128 balances should be flagged.
#[test]
fn sor002_vulnerable_raw_arithmetic() {
    assert_rule_snapshot!(Sor002, "SOR002", "vulnerable", "sor002_raw_arithmetic.rs");
}

/// checked_add / checked_sub should not be flagged.
#[test]
fn sor002_safe_checked_arithmetic() {
    assert_rule_snapshot!(Sor002, "SOR002", "safe", "sor002_checked_arithmetic.rs");
}

/// Vulnerable manifest: [profile.release] without overflow-checks = true.
#[test]
fn sor002_manifest_missing_overflow_checks() {
    use harness::workspace_root;
    let manifest = workspace_root().join("tests/fixtures/vulnerable/sor002_manifest.toml");
    let finding = check_cargo_profile(&manifest, Severity::Warning);
    assert!(
        finding.is_some(),
        "expected a finding for missing overflow-checks"
    );
    let f = finding.unwrap();
    assert_eq!(f.rule_id, "SOR002");
    assert!(f.message.contains("overflow-checks"));
}

/// Safe manifest: overflow-checks = true should produce no finding.
#[test]
fn sor002_manifest_has_overflow_checks() {
    use harness::workspace_root;
    let manifest = workspace_root().join("tests/fixtures/safe/sor002_manifest.toml");
    let finding = check_cargo_profile(&manifest, Severity::Warning);
    assert!(
        finding.is_none(),
        "expected no finding when overflow-checks = true"
    );
}
