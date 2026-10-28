//! Snapshot tests for SOR005 `unbounded-storage-growth`.

mod harness;

use lint_rules::sor005::Sor005;

/// The README's vulnerable example: Vec loaded, grown, stored without a cap.
#[test]
fn sor005_vulnerable_unbounded_vec() {
    assert_rule_snapshot!(Sor005, "SOR005", "vulnerable", "sor005_unbounded_vec.rs");
}

/// Growth delegated to a helper function — the helper should be flagged.
#[test]
fn sor005_vulnerable_helper_growth() {
    assert_rule_snapshot!(Sor005, "SOR005", "vulnerable", "sor005_helper_growth.rs");
}

/// Per-item keys (safe pattern) — must not trigger.
#[test]
fn sor005_safe_per_item_keys() {
    assert_rule_snapshot!(Sor005, "SOR005", "safe", "sor005_per_item_keys.rs");
}

/// Explicit len() cap with typed error — must not trigger.
#[test]
fn sor005_safe_len_cap() {
    assert_rule_snapshot!(Sor005, "SOR005", "safe", "sor005_len_cap.rs");
}

/// Collection not written back — must not trigger.
#[test]
fn sor005_safe_not_written_back() {
    assert_rule_snapshot!(Sor005, "SOR005", "safe", "sor005_not_written_back.rs");
}
