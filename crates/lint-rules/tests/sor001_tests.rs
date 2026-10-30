//! Snapshot tests for SOR001 `missing-require-auth`.
//!
//! Each test runs the rule against one fixture file and asserts the findings
//! match the stored insta snapshot.  Snapshots are reviewed and accepted with
//! `cargo insta review`.
//!
//! Fixture naming convention: `sor<NNN>_<case>.rs` under
//! `tests/fixtures/{vulnerable,safe}/`.

mod harness;

use lint_rules::sor001::Sor001;

/// The README's primary vulnerable example: `transfer` with no auth.
#[test]
fn sor001_vulnerable_basic() {
    assert_rule_snapshot!(Sor001, "SOR001", "vulnerable", "sor001_basic.rs");
}

/// A `remove` call without auth should also be flagged.
#[test]
fn sor001_vulnerable_remove() {
    assert_rule_snapshot!(Sor001, "SOR001", "vulnerable", "sor001_remove.rs");
}

/// Authed `transfer` must not produce findings.
#[test]
fn sor001_safe_authed() {
    assert_rule_snapshot!(Sor001, "SOR001", "safe", "sor001_authed.rs");
}

/// Read-only entrypoint must not produce findings.
#[test]
fn sor001_safe_readonly() {
    assert_rule_snapshot!(Sor001, "SOR001", "safe", "sor001_readonly.rs");
}

/// `require_auth_for_args` must count as auth.
#[test]
fn sor001_safe_auth_for_args() {
    assert_rule_snapshot!(Sor001, "SOR001", "safe", "sor001_auth_for_args.rs");
}

/// Auth via private helper must not be flagged (call-graph credit).
#[test]
fn sor001_safe_auth_via_helper() {
    assert_rule_snapshot!(Sor001, "SOR001", "safe", "sor001_auth_via_helper.rs");
}

/// __constructor is exempt from require-auth (corpus FP fix).
#[test]
fn sor001_safe_constructor() {
    assert_rule_snapshot!(Sor001, "SOR001", "safe", "sor001_constructor.rs");
}
