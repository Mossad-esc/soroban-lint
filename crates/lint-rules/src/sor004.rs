//! SOR004 `critical-data-in-temporary` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `critical-data-in-temporary` rule.
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

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
