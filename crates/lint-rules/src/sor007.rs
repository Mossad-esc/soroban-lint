//! SOR007 `unprotected-initialize` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `unprotected-initialize` rule.
#[derive(Debug, Default)]
pub struct Sor007;

impl Rule for Sor007 {
    fn id(&self) -> &'static str {
        "SOR007"
    }

    fn name(&self) -> &'static str {
        "unprotected-initialize"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
