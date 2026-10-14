//! SOR003 `missing-ttl-extension` (placeholder).

use lint_core::{Finding, Rule, RuleContext, Severity};

/// Placeholder implementation of the `missing-ttl-extension` rule.
#[derive(Debug, Default)]
pub struct Sor003;

impl Rule for Sor003 {
    fn id(&self) -> &'static str {
        "SOR003"
    }

    fn name(&self) -> &'static str {
        "missing-ttl-extension"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
        Vec::new()
    }
}
