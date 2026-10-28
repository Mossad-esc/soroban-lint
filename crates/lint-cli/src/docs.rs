//! Rule documentation embedded at compile time via `include_str!`.
//!
//! Each docs page lives at `docs/rules/<ID>.md` in the workspace root. By
//! embedding them here the installed binary works without the repository
//! present.

/// Returns the Markdown documentation for `rule_id` (upper-case, e.g.
/// `"SOR001"`), or `None` when no embedded page exists for that ID.
#[must_use]
pub fn rule_doc(rule_id: &str) -> Option<&'static str> {
    match rule_id {
        "SOR001" => Some(include_str!("../../../docs/rules/SOR001.md")),
        "SOR002" => Some(include_str!("../../../docs/rules/SOR002.md")),
        "SOR003" => Some(include_str!("../../../docs/rules/SOR003.md")),
        "SOR004" => Some(include_str!("../../../docs/rules/SOR004.md")),
        "SOR005" => Some(include_str!("../../../docs/rules/SOR005.md")),
        "SOR006" => Some(include_str!("../../../docs/rules/SOR006.md")),
        "SOR007" => Some(include_str!("../../../docs/rules/SOR007.md")),
        "SOR008" => Some(include_str!("../../../docs/rules/SOR008.md")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lint_rules::builtin_registry;

    /// Every registered rule must have an embedded docs page.
    #[test]
    fn every_rule_has_a_docs_page() {
        let registry = builtin_registry();
        let mut missing = Vec::new();
        for entry in registry.iter() {
            if rule_doc(entry.id()).is_none() {
                missing.push(entry.id());
            }
        }
        assert!(
            missing.is_empty(),
            "rules missing embedded docs pages: {}",
            missing.join(", ")
        );
    }
}
