//! SOR008 `std-or-float-usage` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `std-or-float-usage` rule.
#[derive(Debug, Default)]
pub struct Sor008;

impl Rule for Sor008 {
    fn id(&self) -> &'static str {
        "SOR008"
    }

    fn name(&self) -> &'static str {
        "std-or-float-usage"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
