//! Snapshot tests for SOR008 `std-or-float-usage`.

mod harness;

use lint_rules::sor008::Sor008;

/// Vulnerable: missing no_std, std import, and f64 float usage.
#[test]
fn sor008_vulnerable_std_and_float() {
    assert_rule_snapshot!(Sor008, "SOR008", "vulnerable", "sor008_std_and_float.rs");
}

/// Safe: #![no_std] with integer math — no findings.
#[test]
fn sor008_safe_no_std_integer_math() {
    assert_rule_snapshot!(Sor008, "SOR008", "safe", "sor008_no_std_integer_math.rs");
}

/// Safe: #![cfg_attr(not(test), no_std)] is treated as no_std present.
#[test]
fn sor008_safe_cfg_attr_no_std() {
    assert_rule_snapshot!(Sor008, "SOR008", "safe", "sor008_cfg_attr_no_std.rs");
}
