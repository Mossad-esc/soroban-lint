//! Snapshot tests for the `pretty` reporter.
//!
//! Timing is stripped from snapshots by passing a fixed [`Duration`] in every
//! test report.  The `use_color = false` flag ensures ANSI codes don't appear
//! in snapshots.

use std::path::{Path, PathBuf};
use std::time::Duration;

use lint_core::{Finding, Severity};
use lint_report::pretty::PrettyReporter;
use lint_report::{Report, Reporter};

/// Returns the workspace root (two levels up from `crates/lint-report`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(Path::parent) // workspace root
        .expect("workspace root")
        .to_path_buf()
}

/// A fixed elapsed duration used in all tests so snapshot timing is stable.
const FIXED_ELAPSED: Duration = Duration::from_millis(40);

fn sor001_error(file: &str, line: usize, col: usize) -> Finding {
    Finding::new(
        "SOR001",
        "missing-require-auth",
        Severity::Error,
        "entrypoint `transfer` writes to storage but never calls `require_auth`",
        PathBuf::from(file),
        line,
        col,
    )
    .with_help("call `from.require_auth()` before mutating state")
    .with_docs_url("https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR001.md")
}

fn sor002_warning(file: &str, line: usize, col: usize) -> Finding {
    Finding::new(
        "SOR002",
        "unchecked-arithmetic",
        Severity::Warning,
        "raw arithmetic on integer value",
        PathBuf::from(file),
        line,
        col,
    )
    .with_help("use `checked_add` and handle overflow explicitly")
}

fn report(findings: Vec<Finding>, files: usize) -> Report {
    Report {
        findings,
        files_scanned: files,
        elapsed: FIXED_ELAPSED,
        baselined_count: 0,
    }
}

/// Zero findings → "No issues found …" summary only.
#[test]
fn pretty_zero_findings() {
    let output = PrettyReporter::new(false)
        .render(&report(vec![], 3))
        .unwrap();
    insta::assert_snapshot!(output);
}

/// One error finding with a real fixture file so the snippet can be rendered.
#[test]
fn pretty_one_finding() {
    // Line 23, col 12 matches the vulnerable sor001_basic.rs fixture.
    // Use absolute path so the reporter can read the source for the snippet.
    let fixture = workspace_root().join("tests/fixtures/vulnerable/sor001_basic.rs");
    let finding = sor001_error(&fixture.to_string_lossy(), 23, 12);
    let output = PrettyReporter::new(false)
        .render(&report(vec![finding], 1))
        .unwrap();
    // Strip the absolute prefix so the snapshot is portable.
    let root_str = workspace_root().to_string_lossy().replace('\\', "/");
    let portable = output.replace(&root_str, "<workspace>");
    insta::assert_snapshot!(portable);
}

/// Multiple findings across two files; verifies summary pluralisation and
/// ordering.
#[test]
fn pretty_multiple_findings() {
    let root = workspace_root();
    let basic = root.join("tests/fixtures/vulnerable/sor001_basic.rs");
    let remove = root.join("tests/fixtures/vulnerable/sor001_remove.rs");
    let findings = vec![
        sor001_error(&basic.to_string_lossy(), 23, 12),
        sor001_error(&remove.to_string_lossy(), 24, 12),
        sor002_warning(&basic.to_string_lossy(), 31, 9),
    ];
    let output = PrettyReporter::new(false)
        .render(&report(findings, 2))
        .unwrap();
    let root_str = root.to_string_lossy().replace('\\', "/");
    let portable = output.replace(&root_str, "<workspace>");
    insta::assert_snapshot!(portable);
}
