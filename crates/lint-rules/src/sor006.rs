//! SOR006 `panic-in-entrypoint` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `panic-in-entrypoint` rule.
#[derive(Debug, Default)]
pub struct Sor006;

impl Rule for Sor006 {
    fn id(&self) -> &'static str {
        "SOR006"
    }

    fn name(&self) -> &'static str {
        "panic-in-entrypoint"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
