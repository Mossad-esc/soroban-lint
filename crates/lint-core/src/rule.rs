//! The `Rule` trait and the context passed to rules.

use std::path::Path;

use syn::File;

use crate::finding::Finding;
use crate::model::ContractModel;
use crate::severity::Severity;

/// Per-rule resolved configuration.
///
/// This is a placeholder while configuration loading is not implemented yet;
/// rule-specific options are added when `soroban-lint.toml` parsing lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleConfig {
    /// Whether the rule is enabled.
    pub enabled: bool,
    /// Optional severity override; `None` uses the rule's default severity.
    pub severity: Option<Severity>,
}

impl Default for RuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            severity: None,
        }
    }
}

impl RuleConfig {
    /// Returns the configured severity, falling back to `default`.
    #[must_use]
    pub fn severity_or(&self, default: Severity) -> Severity {
        self.severity.unwrap_or(default)
    }
}

/// Everything a rule needs to analyze one source file.
///
/// The context borrows the parsed AST, the source text, the contract model, and
/// the resolved rule configuration. Rules must not mutate any of it.
pub struct RuleContext<'a> {
    ast: &'a File,
    path: &'a Path,
    source: &'a str,
    model: &'a ContractModel<'a>,
    config: &'a RuleConfig,
}

impl<'a> RuleContext<'a> {
    /// Creates a rule context from its inputs.
    pub fn new(
        ast: &'a File,
        path: &'a Path,
        source: &'a str,
        model: &'a ContractModel<'a>,
        config: &'a RuleConfig,
    ) -> Self {
        Self {
            ast,
            path,
            source,
            model,
            config,
        }
    }

    /// The parsed abstract syntax tree of the file.
    #[must_use]
    pub fn ast(&self) -> &'a File {
        self.ast
    }

    /// The path of the file being analyzed.
    #[must_use]
    pub fn path(&self) -> &'a Path {
        self.path
    }

    /// The raw source text of the file.
    #[must_use]
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// The contract model built from this file.
    #[must_use]
    pub fn model(&self) -> &'a ContractModel<'a> {
        self.model
    }

    /// The resolved configuration for the running rule.
    #[must_use]
    pub fn config(&self) -> &'a RuleConfig {
        self.config
    }
}

/// A lint rule.
///
/// The interface matches the example in the README's "Writing a New Rule"
/// section: an identifier, a name, a default severity, and a `check` method
/// that returns findings for one file.
pub trait Rule: Send + Sync {
    /// The rule identifier, for example `SOR001`.
    fn id(&self) -> &'static str;

    /// The kebab-case rule name, for example `missing-require-auth`.
    fn name(&self) -> &'static str;

    /// The built-in severity, used when configuration does not override it.
    fn default_severity(&self) -> Severity;

    /// Analyzes one file and returns the findings produced by this rule.
    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Finding>;
}
