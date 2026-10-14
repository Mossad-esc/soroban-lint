//! Registry of the rules available to the CLI.

use crate::rule::Rule;
use crate::severity::Severity;

/// Whether a rule is fully implemented or only a registered placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuleStatus {
    /// The rule has a real implementation and tests.
    Implemented,
    /// The rule is registered for catalog purposes but does not analyze yet.
    Planned,
}

impl RuleStatus {
    /// The lowercase representation shown in tables.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Planned => "planned",
        }
    }
}

/// A rule together with its catalog metadata.
pub struct RegisteredRule {
    rule: Box<dyn Rule>,
    category: &'static str,
    status: RuleStatus,
}

impl RegisteredRule {
    /// The rule identifier, for example `SOR001`.
    #[must_use]
    pub fn id(&self) -> &'static str {
        self.rule.id()
    }

    /// The kebab-case rule name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.rule.name()
    }

    /// The built-in severity of the rule.
    #[must_use]
    pub fn default_severity(&self) -> Severity {
        self.rule.default_severity()
    }

    /// The catalog category, for example `Access control`.
    #[must_use]
    pub fn category(&self) -> &'static str {
        self.category
    }

    /// Whether the rule is implemented or planned.
    #[must_use]
    pub fn status(&self) -> RuleStatus {
        self.status
    }

    /// The underlying rule implementation.
    #[must_use]
    pub fn rule(&self) -> &dyn Rule {
        self.rule.as_ref()
    }
}

/// An ordered collection of rules with id-based lookup.
#[derive(Default)]
pub struct RuleRegistry {
    rules: Vec<RegisteredRule>,
}

impl RuleRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a rule together with its category and status.
    pub fn register(&mut self, rule: Box<dyn Rule>, category: &'static str, status: RuleStatus) {
        self.rules.push(RegisteredRule {
            rule,
            category,
            status,
        });
    }

    /// Looks up a rule by its exact identifier.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&RegisteredRule> {
        self.rules.iter().find(|entry| entry.id() == id)
    }

    /// Returns `true` when `id` is a registered rule identifier.
    #[must_use]
    pub fn is_known_id(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    /// Iterates over the registered rules in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &RegisteredRule> {
        self.rules.iter()
    }

    /// The number of registered rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the registry contains no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::Finding;
    use crate::rule::RuleContext;

    struct TestRule {
        id: &'static str,
        name: &'static str,
        severity: Severity,
    }

    impl Rule for TestRule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn default_severity(&self) -> Severity {
            self.severity
        }

        fn check(&self, _ctx: &RuleContext<'_>) -> Vec<Finding> {
            Vec::new()
        }
    }

    fn test_rule(id: &'static str) -> Box<dyn Rule> {
        Box::new(TestRule {
            id,
            name: "test-rule",
            severity: Severity::Warning,
        })
    }

    #[test]
    fn lookup_returns_registered_rule() {
        let mut registry = RuleRegistry::new();
        registry.register(test_rule("SOR001"), "Access control", RuleStatus::Planned);

        let entry = registry.get("SOR001").expect("rule should be registered");
        assert_eq!(entry.name(), "test-rule");
        assert_eq!(entry.category(), "Access control");
        assert_eq!(entry.status(), RuleStatus::Planned);
        assert_eq!(entry.default_severity(), Severity::Warning);
    }

    #[test]
    fn unknown_id_is_not_found() {
        let registry = RuleRegistry::new();
        assert!(registry.get("SOR999").is_none());
        assert!(!registry.is_known_id("SOR999"));
    }

    #[test]
    fn iterates_in_registration_order() {
        let mut registry = RuleRegistry::new();
        registry.register(test_rule("SOR001"), "Access control", RuleStatus::Planned);
        registry.register(test_rule("SOR002"), "Arithmetic", RuleStatus::Planned);

        let ids: Vec<&str> = registry.iter().map(RegisteredRule::id).collect();
        assert_eq!(ids, vec!["SOR001", "SOR002"]);
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn new_registry_is_empty() {
        let registry = RuleRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }
}
