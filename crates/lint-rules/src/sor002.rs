//! SOR002 `unchecked-arithmetic` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `unchecked-arithmetic` rule.
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

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
