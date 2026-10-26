//! Tests that every *implemented* rule has at least one fixture in both
//! `tests/fixtures/vulnerable/` and `tests/fixtures/safe/`.
//!
//! If a rule is registered as [`RuleStatus::Implemented`] but is missing a
//! fixture file whose name starts with `sor<NNN>` (lower-case), this test
//! fails and names the offending rule.

mod harness;

use std::collections::HashSet;
use std::path::Path;

use lint_core::RuleStatus;
use lint_rules::builtin_registry;

/// Collects all fixture basenames under `tests/fixtures/<subdir>/` whose names
/// start with the lower-case rule prefix (e.g. `sor001`).
fn fixture_prefixes(subdir: &str) -> HashSet<String> {
    let root = harness::workspace_root();
    let dir = root.join("tests").join("fixtures").join(subdir);
    let mut prefixes = HashSet::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return prefixes,
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_lowercase();
        if name_str.ends_with(".rs") {
            // Extract the `sorNNN` prefix (first 6 chars).
            if let Some(prefix) = name_str.get(..6) {
                if prefix.starts_with("sor") {
                    prefixes.insert(prefix.to_owned());
                }
            }
        }
    }
    prefixes
}

#[test]
fn every_implemented_rule_has_vulnerable_and_safe_fixtures() {
    let registry = builtin_registry();
    let vulnerable = fixture_prefixes("vulnerable");
    let safe = fixture_prefixes("safe");

    let mut missing: Vec<String> = Vec::new();

    for entry in registry.iter() {
        if entry.status() != RuleStatus::Implemented {
            continue;
        }
        // Normalise: "SOR001" → "sor001"
        let prefix = entry.id().to_lowercase();
        if !vulnerable.contains(&prefix) {
            missing.push(format!(
                "{}: missing vulnerable fixture (tests/fixtures/vulnerable/{}*.rs)",
                entry.id(),
                prefix
            ));
        }
        if !safe.contains(&prefix) {
            missing.push(format!(
                "{}: missing safe fixture (tests/fixtures/safe/{}*.rs)",
                entry.id(),
                prefix
            ));
        }
    }

    assert!(
        missing.is_empty(),
        "implemented rules are missing fixtures:\n{}",
        missing.join("\n")
    );
}

/// Confirm the harness itself can locate the workspace root and fixture dirs.
#[test]
fn workspace_root_contains_tests_fixtures() {
    let root = harness::workspace_root();
    let fixtures = root.join("tests").join("fixtures");
    assert!(
        fixtures.is_dir(),
        "expected tests/fixtures/ to exist at {}",
        fixtures.display()
    );
    assert!(
        fixtures.join("vulnerable").is_dir(),
        "expected tests/fixtures/vulnerable/"
    );
    assert!(
        fixtures.join("safe").is_dir(),
        "expected tests/fixtures/safe/"
    );
}

/// Confirm fixture_path resolves to an existing file (uses the existing
/// minimal_token.rs in safe/).
#[test]
fn fixture_path_resolves_existing_file() {
    let path = harness::fixture_path("safe", "minimal_token.rs");
    assert!(
        path.is_file(),
        "expected fixture to exist at {}",
        path.display()
    );
}

/// Verify that the harness correctly reports no findings for a trivially safe
/// fixture when run against SOR001 (which is implemented and will find nothing
/// in a read-only contract).
#[test]
fn run_rule_on_fixture_returns_empty_for_non_contract_file() {
    // minimal_token.rs has auth on every mutating entrypoint; SOR001 should
    // return no findings.
    let rule = lint_rules::sor001::Sor001;
    let findings = harness::run_rule_on_fixture(&rule, "safe", "minimal_token.rs");
    // We assert the shape here (could be non-empty if the fixture has issues);
    // the real snapshot tests live in sor001_tests.rs.
    for finding in &findings {
        eprintln!("unexpected finding: {finding:?}");
    }
    assert!(
        findings.is_empty(),
        "expected no findings for minimal_token.rs"
    );
}

/// Confirm the fixture path helper uses the right separator on all platforms.
#[test]
fn fixture_path_uses_system_separator() {
    let path = harness::fixture_path("vulnerable", "sor001_basic.rs");
    // The path should end in the correct filename regardless of OS.
    assert!(
        path.ends_with(Path::new("sor001_basic.rs")),
        "unexpected path: {}",
        path.display()
    );
}
