//! A single issue reported by a rule.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::severity::Severity;

/// One finding produced by a rule.
///
/// The struct is serializable so the JSON and SARIF reporters can consume it
/// directly in later milestones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Rule identifier, for example `SOR001`.
    pub rule_id: String,
    /// Kebab-case rule name, for example `missing-require-auth`.
    pub rule_name: String,
    /// Effective severity after configuration and CLI overrides.
    pub severity: Severity,
    /// Human-readable description of the problem.
    pub message: String,
    /// File in which the finding was produced.
    pub file: PathBuf,
    /// 1-based line number of the primary span.
    pub line: usize,
    /// 1-based column number of the primary span.
    pub column: usize,
    /// Optional actionable suggestion.
    pub help: Option<String>,
    /// Optional documentation URL for the rule.
    pub docs_url: Option<String>,
}

impl Finding {
    /// Creates a finding without help text or a documentation URL.
    pub fn new(
        rule_id: impl Into<String>,
        rule_name: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        file: impl Into<PathBuf>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            rule_name: rule_name.into(),
            severity,
            message: message.into(),
            file: file.into(),
            line,
            column,
            help: None,
            docs_url: None,
        }
    }

    /// Sets the actionable help text.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Sets the documentation URL for the rule.
    #[must_use]
    pub fn with_docs_url(mut self, url: impl Into<String>) -> Self {
        self.docs_url = Some(url.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_leaves_optionals_empty() {
        let finding = Finding::new(
            "SOR001",
            "missing-require-auth",
            Severity::Error,
            "missing auth",
            "src/lib.rs",
            1,
            2,
        );
        assert_eq!(finding.rule_id, "SOR001");
        assert_eq!(finding.severity, Severity::Error);
        assert!(finding.help.is_none());
        assert!(finding.docs_url.is_none());
    }

    #[test]
    fn builder_helpers_populate_optionals() {
        let finding = Finding::new(
            "SOR001",
            "missing-require-auth",
            Severity::Error,
            "missing auth",
            "src/lib.rs",
            1,
            2,
        )
        .with_help("call `from.require_auth()`")
        .with_docs_url("https://example.test/rules/SOR001");
        assert_eq!(finding.help.as_deref(), Some("call `from.require_auth()`"));
        assert_eq!(
            finding.docs_url.as_deref(),
            Some("https://example.test/rules/SOR001")
        );
    }
}
