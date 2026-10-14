//! SOR005 `unbounded-storage-growth` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `unbounded-storage-growth` rule.
#[derive(Debug, Default)]
pub struct Sor005;

impl Rule for Sor005 {
    fn id(&self) -> &'static str {
        "SOR005"
    }

    fn name(&self) -> &'static str {
        "unbounded-storage-growth"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
