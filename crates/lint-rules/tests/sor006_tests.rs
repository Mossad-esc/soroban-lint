//! Snapshot tests for SOR006 `panic-in-entrypoint`.

mod harness;

use lint_rules::sor006::Sor006;

/// Vulnerable: unwrap, expect, panic!, and direct indexing.
#[test]
fn sor006_vulnerable_panics() {
    assert_rule_snapshot!(Sor006, "SOR006", "vulnerable", "sor006_panics.rs");
}

/// Safe: ok_or with #[contracterror] — no panics.
#[test]
fn sor006_safe_ok_or() {
    assert_rule_snapshot!(Sor006, "SOR006", "safe", "sor006_ok_or.rs");
}

/// Safe: unwrap_or_default and unwrap_or — no panics.
#[test]
fn sor006_safe_unwrap_or() {
    assert_rule_snapshot!(Sor006, "SOR006", "safe", "sor006_unwrap_or.rs");
}

/// Safe: unwrap() only in #[cfg(test)] code — not flagged.
#[test]
fn sor006_safe_test_only_unwrap() {
    assert_rule_snapshot!(Sor006, "SOR006", "safe", "sor006_test_only_unwrap.rs");
}
