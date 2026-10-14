//! SOR001 `missing-require-auth` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `missing-require-auth` rule.
#[derive(Debug, Default)]
pub struct Sor001;

impl Rule for Sor001 {
    fn id(&self) -> &'static str {
        "SOR001"
    }

    fn name(&self) -> &'static str {
        "missing-require-auth"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
