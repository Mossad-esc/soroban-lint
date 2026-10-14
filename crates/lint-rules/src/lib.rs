//! Built-in lint rules for `soroban-lint`.
//!
//! Each rule lives in its own module (`sor001`, `sor002`, ...). The
//! [`builtin_registry`] function wires the catalog together with the exact
//! identifiers, names, default severities, and categories from the README.

pub mod sor001;
pub mod sor002;
pub mod sor003;
pub mod sor004;
pub mod sor005;
pub mod sor006;
pub mod sor007;
pub mod sor008;

use lint_core::{RuleRegistry, RuleStatus};

/// Builds a registry containing every built-in rule.
///
/// All rules are currently registered as [`RuleStatus::Planned`]; each rule is
/// flipped to [`RuleStatus::Implemented`] as its implementation lands.
#[must_use]
pub fn builtin_registry() -> RuleRegistry {
    let mut registry = RuleRegistry::new();

    registry.register(
        Box::new(sor001::Sor001),
        "Access control",
        RuleStatus::Planned,
    );
    registry.register(Box::new(sor002::Sor002), "Arithmetic", RuleStatus::Planned);
    registry.register(
        Box::new(sor003::Sor003),
        "Storage / TTL",
        RuleStatus::Planned,
    );
    registry.register(Box::new(sor004::Sor004), "Storage", RuleStatus::Planned);
    registry.register(
        Box::new(sor005::Sor005),
        "Storage / DoS",
        RuleStatus::Planned,
    );
    registry.register(
        Box::new(sor006::Sor006),
        "Error handling",
        RuleStatus::Planned,
    );
    registry.register(
        Box::new(sor007::Sor007),
        "Access control",
        RuleStatus::Planned,
    );
    registry.register(
        Box::new(sor008::Sor008),
        "Compatibility",
        RuleStatus::Planned,
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::Severity;

    // (id, name, default severity, category)
    const CATALOG: &[(&str, &str, Severity, &str)] = &[
        (
            "SOR001",
            "missing-require-auth",
            Severity::Error,
            "Access control",
        ),
        (
            "SOR002",
            "unchecked-arithmetic",
            Severity::Warning,
            "Arithmetic",
        ),
        (
            "SOR003",
            "missing-ttl-extension",
            Severity::Warning,
            "Storage / TTL",
        ),
        (
            "SOR004",
            "critical-data-in-temporary",
            Severity::Error,
            "Storage",
        ),
        (
            "SOR005",
            "unbounded-storage-growth",
            Severity::Warning,
            "Storage / DoS",
        ),
        (
            "SOR006",
            "panic-in-entrypoint",
            Severity::Warning,
            "Error handling",
        ),
        (
            "SOR007",
            "unprotected-initialize",
            Severity::Error,
            "Access control",
        ),
        (
            "SOR008",
            "std-or-float-usage",
            Severity::Error,
            "Compatibility",
        ),
    ];

    #[test]
    fn registers_every_catalog_rule() {
        let registry = builtin_registry();
        assert_eq!(registry.len(), CATALOG.len());

        for (id, name, severity, category) in CATALOG {
            let entry = registry.get(id).unwrap_or_else(|| panic!("missing {id}"));
            assert_eq!(entry.name(), *name, "name mismatch for {id}");
            assert_eq!(
                entry.default_severity(),
                *severity,
                "severity mismatch for {id}"
            );
            assert_eq!(entry.category(), *category, "category mismatch for {id}");
            assert_eq!(entry.status(), RuleStatus::Planned);
        }
    }

    #[test]
    fn registration_order_matches_catalog() {
        let registry = builtin_registry();
        let ids: Vec<&str> = registry.iter().map(|entry| entry.id()).collect();
        let expected: Vec<&str> = CATALOG.iter().map(|(id, ..)| *id).collect();
        assert_eq!(ids, expected);
    }
}
