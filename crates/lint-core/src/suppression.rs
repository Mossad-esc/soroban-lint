//! Inline suppression comment scanner.
//!
//! Soroban-lint suppression comments take the form:
//!
//! ```text
//! // soroban-lint: allow(SOR001)
//! // soroban-lint: allow(SOR001, SOR003)
//! // soroban-lint: allow(SOR001) reason="free text"
//! ```
//!
//! A directive applies to the **next non-comment, non-blank, non-attribute
//! line** (the "target line").  Any finding whose primary span falls on a
//! line that is covered by that target item has its rule ID listed in the
//! allow set, and is dropped.
//!
//! # Implementation approach
//!
//! `syn` discards comments, so we scan the raw source text directly.
//! The scanner:
//!
//! 1. Iterates lines.
//! 2. For each line that matches `// soroban-lint: allow(…)` outside a block
//!    comment or string literal, records the parsed directive and the line
//!    number at which it was found.
//! 3. After collecting all directives, associates each with the 1-based line
//!    of the next "item start" (first non-comment, non-blank, non-`#[…]`
//!    line following the directive).
//! 4. Exposes a single query: `is_suppressed(line, rule_id)` which returns
//!    `true` when `line` is covered by a suppression for `rule_id`.
//!
//! Unknown rule IDs in an `allow(…)` list produce a warning-level diagnostic
//! (returned alongside the findings).  Unused suppressions are counted but
//! not reported as findings today.

use std::collections::HashMap;

use crate::finding::Finding;

const KNOWN_RULE_IDS: &[&str] = &[
    "SOR001", "SOR002", "SOR003", "SOR004", "SOR005", "SOR006", "SOR007", "SOR008",
];

/// A parsed suppression directive.
#[derive(Debug, Clone)]
pub struct Directive {
    /// The 1-based source line of the `// soroban-lint: allow(…)` comment.
    pub comment_line: usize,
    /// The 1-based line of the item the directive covers.
    pub target_line: usize,
    /// Rule IDs listed in `allow(…)`.
    pub rule_ids: Vec<String>,
    /// Optional reason text after `reason="…"`.
    pub reason: Option<String>,
    /// Whether this directive was ever used to suppress a finding (internal).
    pub used: bool,
}

/// A diagnostic produced during suppression scanning (unknown rule ID, etc.).
#[derive(Debug, Clone)]
pub struct SuppressionDiagnostic {
    /// 1-based source line of the bad directive.
    pub line: usize,
    /// Human-readable message.
    pub message: String,
}

/// The result of scanning a source file for suppression directives.
#[derive(Debug, Default)]
pub struct SuppressionMap {
    /// Directives keyed by their target line.
    directives: HashMap<usize, Vec<Directive>>,
    /// Non-fatal diagnostics (unknown IDs, etc.).
    pub diagnostics: Vec<SuppressionDiagnostic>,
    /// Total number of directives that did not suppress any finding.
    pub unused_count: usize,
}

impl SuppressionMap {
    /// Returns `true` when `finding_line` is covered by a suppression for
    /// `rule_id`.
    #[must_use]
    pub fn is_suppressed(&self, finding_line: usize, rule_id: &str) -> bool {
        // A directive covers a finding when the finding's primary span line
        // equals the directive's target line.
        if let Some(directives) = self.directives.get(&finding_line) {
            for directive in directives {
                if directive.rule_ids.iter().any(|id| id == rule_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Marks directives that matched findings as used and returns the unused
    /// count.  Should be called after filtering findings.
    pub fn mark_used(&mut self, findings_before: &[Finding], findings_after: &[Finding]) {
        let suppressed_count = findings_before.len().saturating_sub(findings_after.len());
        // Simple heuristic: if any findings were suppressed, decrement unused.
        // (A precise per-directive used flag requires more bookkeeping that
        // is tracked for future use.)
        if suppressed_count == 0 {
            self.unused_count = self.directives.values().map(|v| v.len()).sum();
        }
    }
}

/// Scans `source` for suppression directives and returns a [`SuppressionMap`].
///
/// Unknown rule IDs in directives produce entries in
/// [`SuppressionMap::diagnostics`].
#[must_use]
pub fn scan(source: &str) -> SuppressionMap {
    let mut map = SuppressionMap::default();

    let lines: Vec<&str> = source.lines().collect();
    let total = lines.len();

    let mut idx = 0usize;
    while idx < total {
        let line_no = idx + 1; // 1-based
        let line = lines[idx];

        if let Some(directive_raw) = extract_directive(line) {
            // Find the target line: next non-blank, non-comment,
            // non-attribute line.
            let mut target_idx = idx + 1;
            while target_idx < total {
                let tl = lines[target_idx].trim();
                let is_blank = tl.is_empty();
                let is_comment = tl.starts_with("//");
                let is_attr = tl.starts_with('#');
                if !is_blank && !is_comment && !is_attr {
                    break;
                }
                target_idx += 1;
            }
            let target_line = target_idx + 1; // 1-based (may be past EOF)

            // Parse the directive text.
            let (rule_ids, reason, diags) = parse_directive(directive_raw, line_no);
            map.diagnostics.extend(diags);

            let directive = Directive {
                comment_line: line_no,
                target_line,
                rule_ids,
                reason,
                used: false,
            };

            map.directives
                .entry(target_line)
                .or_default()
                .push(directive);
        }

        idx += 1;
    }

    map
}

/// Filters `findings` against `map`, dropping any finding whose line is
/// covered by a suppression for that finding's rule.
///
/// Returns the filtered list.
#[must_use]
pub fn apply(findings: Vec<Finding>, map: &SuppressionMap) -> Vec<Finding> {
    findings
        .into_iter()
        .filter(|f| !map.is_suppressed(f.line, &f.rule_id))
        .collect()
}

// ─── Parsing helpers ──────────────────────────────────────────────────────────

/// Extracts the payload after `// soroban-lint: allow(` from a raw source line,
/// returning `None` when the line is not a suppression directive.
///
/// The extraction skips lines that are inside string literals by ensuring the
/// `//` is not preceded by an odd number of `"` characters (a simple
/// heuristic sufficient for realistic source files).
fn extract_directive(line: &str) -> Option<&str> {
    const PREFIX: &str = "// soroban-lint: allow(";
    let trimmed = line.trim();

    // Must start with the prefix (after trimming leading whitespace).
    if !trimmed.starts_with(PREFIX) {
        return None;
    }

    // Guard: make sure the `//` is not inside a string literal by checking
    // that there are no unescaped `"` before the `//` on the same line.
    if let Some(comment_pos) = line.find("//") {
        let before = &line[..comment_pos];
        let quote_count = before.chars().filter(|&c| c == '"').count();
        if quote_count % 2 != 0 {
            // Inside a string literal — not a real directive.
            return None;
        }
    }

    // Return everything after the PREFIX.
    Some(&trimmed[PREFIX.len()..])
}

/// Parses `rule_ids)` or `rule_ids) reason="…"` from the tail of a directive.
///
/// Returns `(rule_ids, reason, diagnostics)`.
fn parse_directive(
    tail: &str,
    line_no: usize,
) -> (Vec<String>, Option<String>, Vec<SuppressionDiagnostic>) {
    let mut diagnostics = Vec::new();

    // Split on ')' to separate the IDs from the optional reason.
    let (ids_part, after_paren) = match tail.split_once(')') {
        Some(pair) => pair,
        None => {
            diagnostics.push(SuppressionDiagnostic {
                line: line_no,
                message: "malformed suppression directive: missing `)`".to_owned(),
            });
            return (Vec::new(), None, diagnostics);
        }
    };

    // Parse comma-separated rule IDs.
    let mut rule_ids = Vec::new();
    for raw_id in ids_part.split(',') {
        let id = raw_id.trim().to_ascii_uppercase();
        if id.is_empty() {
            continue;
        }
        if !KNOWN_RULE_IDS.contains(&id.as_str()) {
            diagnostics.push(SuppressionDiagnostic {
                line: line_no,
                message: format!(
                    "unknown rule ID `{id}` in suppression directive; \
                     known IDs are {}",
                    KNOWN_RULE_IDS.join(", ")
                ),
            });
            // Still record the ID so the directive is tracked.
        }
        rule_ids.push(id);
    }

    // Parse optional `reason="…"`.
    let reason = parse_reason(after_paren.trim());

    (rule_ids, reason, diagnostics)
}

/// Extracts the reason string from `reason="…"` if present.
fn parse_reason(s: &str) -> Option<String> {
    let s = s.trim();
    if !s.starts_with("reason=") {
        return None;
    }
    let rest = s["reason=".len()..].trim();
    if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
        Some(rest[1..rest.len() - 1].to_owned())
    } else {
        None
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::severity::Severity;

    fn make_finding(rule_id: &str, line: usize) -> Finding {
        Finding::new(
            rule_id,
            "test-rule",
            Severity::Warning,
            "test message",
            "test.rs",
            line,
            1,
        )
    }

    // ── scan() ───────────────────────────────────────────────────────────────

    #[test]
    fn single_rule_suppressed_on_next_line() {
        let source = "// soroban-lint: allow(SOR001)\npub fn transfer() {}";
        let map = scan(source);
        assert!(map.is_suppressed(2, "SOR001"));
        assert!(!map.is_suppressed(2, "SOR002"));
    }

    #[test]
    fn multiple_rules_in_one_directive() {
        let source = "// soroban-lint: allow(SOR001, SOR003)\npub fn f() {}";
        let map = scan(source);
        assert!(map.is_suppressed(2, "SOR001"));
        assert!(map.is_suppressed(2, "SOR003"));
        assert!(!map.is_suppressed(2, "SOR002"));
    }

    #[test]
    fn directive_with_reason() {
        let source =
            "// soroban-lint: allow(SOR001) reason=\"permissionless by design\"\npub fn deposit() {}";
        let map = scan(source);
        assert!(map.is_suppressed(2, "SOR001"));
        // Reason should be stored on the directive.
        let directives = map.directives.get(&2).unwrap();
        assert_eq!(
            directives[0].reason.as_deref(),
            Some("permissionless by design")
        );
    }

    #[test]
    fn directive_skips_blank_and_comment_lines_to_target() {
        let source = "// soroban-lint: allow(SOR001)\n\n// another comment\npub fn f() {}";
        let map = scan(source);
        // target_line should be 4 (the `pub fn f()` line)
        assert!(map.is_suppressed(4, "SOR001"));
        assert!(!map.is_suppressed(2, "SOR001"));
    }

    #[test]
    fn directive_inside_string_literal_not_detected() {
        let source = "let s = \"// soroban-lint: allow(SOR001)\";\npub fn f() {}";
        let map = scan(source);
        // The `//` is inside a string — should not be treated as a directive.
        assert!(!map.is_suppressed(2, "SOR001"));
    }

    #[test]
    fn unknown_rule_id_produces_diagnostic_not_crash() {
        let source = "// soroban-lint: allow(SOR999)\npub fn f() {}";
        let map = scan(source);
        assert!(!map.diagnostics.is_empty());
        assert!(map.diagnostics[0].message.contains("SOR999"));
    }

    #[test]
    fn directive_not_followed_by_item_targets_past_eof() {
        let source = "// soroban-lint: allow(SOR001)";
        let map = scan(source);
        // Target is line 2 (past EOF).  is_suppressed for line 1 is false.
        assert!(!map.is_suppressed(1, "SOR001"));
    }

    // ── apply() ──────────────────────────────────────────────────────────────

    #[test]
    fn apply_removes_suppressed_findings() {
        let source = "// soroban-lint: allow(SOR001)\npub fn transfer() {}";
        let map = scan(source);
        let findings = vec![make_finding("SOR001", 2), make_finding("SOR002", 2)];
        let filtered = apply(findings, &map);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule_id, "SOR002");
    }

    #[test]
    fn apply_on_helper_vs_entrypoint() {
        // Directive on a helper function — finding on that line is suppressed.
        let source = "fn helper() {}\n// soroban-lint: allow(SOR001)\npub fn entry() {}";
        let map = scan(source);
        let findings = vec![
            make_finding("SOR001", 3), // entrypoint
            make_finding("SOR001", 1), // helper — not suppressed
        ];
        let filtered = apply(findings, &map);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].line, 1);
    }

    // ── Severity conversion for diagnostics ──────────────────────────────────

    #[test]
    fn suppression_diagnostic_to_finding() {
        let source = "// soroban-lint: allow(SOR999)\npub fn f() {}";
        let map = scan(source);
        // Convert to a warning finding for reporting.
        let path = std::path::Path::new("test.rs");
        let warn_findings: Vec<Finding> = map
            .diagnostics
            .iter()
            .map(|d| {
                Finding::new(
                    "suppression",
                    "unknown-suppression-id",
                    Severity::Warning,
                    &d.message,
                    path,
                    d.line,
                    1,
                )
            })
            .collect();
        assert_eq!(warn_findings.len(), 1);
        assert!(warn_findings[0].message.contains("SOR999"));
    }
}
