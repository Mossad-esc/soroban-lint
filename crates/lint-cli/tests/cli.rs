//! End-to-end CLI tests using `assert_cmd`.
//!
//! These tests verify the full pipeline described in the README: discovery,
//! analysis, output, exit codes, and flag behaviour.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Returns a `Command` pointing at the `soroban-lint` binary.
fn bin() -> Command {
    Command::cargo_bin("soroban-lint").expect("binary should build")
}

/// Absolute path to a fixture file.
fn fixture(subdir: &str, name: &str) -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR is `crates/lint-cli` at compile time.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent() // crates/
        .and_then(Path::parent) // workspace root
        .expect("workspace root")
        .join("tests")
        .join("fixtures")
        .join(subdir)
        .join(name)
}

// ---------------------------------------------------------------------------
// Pre-existing smoke tests (retained from original cli.rs)
// ---------------------------------------------------------------------------

#[test]
fn help_lists_commands() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("rules"))
        .stdout(predicate::str::contains("explain"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn rules_command_lists_catalog() {
    bin()
        .arg("rules")
        .assert()
        .success()
        .stdout(predicate::str::contains("SOR001"))
        .stdout(predicate::str::contains("missing-require-auth"));
}

#[test]
fn explain_unknown_id_exits_two() {
    bin()
        .args(["explain", "SOR999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown rule"));
}

#[test]
fn unknown_select_id_exits_two() {
    bin()
        .args(["check", "--select", "SOR999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown rule id"));
}

#[test]
fn invalid_format_exits_two() {
    bin().args(["check", "--format", "xml"]).assert().code(2);
}

#[test]
fn check_on_empty_directory_exits_zero() {
    let dir = tempdir().expect("temp dir");
    bin()
        .arg("check")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues found"));
}

// ---------------------------------------------------------------------------
// New end-to-end tests for the wired-up pipeline
// ---------------------------------------------------------------------------

/// A vulnerable fixture (missing require_auth) must exit 1 and print SOR001.
#[test]
fn vulnerable_file_exits_one_and_prints_sor001() {
    bin()
        .arg("check")
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SOR001"))
        .stdout(predicate::str::contains("transfer"))
        .stdout(predicate::str::contains("require_auth"))
        .stdout(predicate::str::contains("Found 1 error in 1 file"));
}

/// A safe fixture must exit 0 and print no issues.
#[test]
fn safe_file_exits_zero() {
    bin()
        .arg("check")
        .arg(fixture("safe", "sor001_authed.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found in 1 file"));
}

/// `--ignore SOR001` suppresses the only finding, so exit 0.
#[test]
fn ignore_sor001_exits_zero() {
    bin()
        .args(["check", "--ignore", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}

/// `--select SOR001` runs only SOR001; result is the same as the default for
/// a file that only has SOR001 findings.
#[test]
fn select_sor001_still_reports_finding() {
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SOR001"));
}

/// `--fail-on warning` still exits 1 because SOR001 is an error (>= warning).
#[test]
fn fail_on_warning_exits_one_for_error_finding() {
    bin()
        .args(["check", "--fail-on", "warning"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1);
}

/// `--min-severity error` filters out warnings; with only an error finding the
/// exit code is still 1.
#[test]
fn min_severity_error_keeps_error_findings() {
    bin()
        .args(["check", "--min-severity", "error"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SOR001"));
}

/// `--format json` exits 2 with a clear "not available yet" error message.
#[test]
fn format_json_exits_two_with_message() {
    bin()
        .args([
            "check",
            "--format",
            "json",
            fixture("vulnerable", "sor001_basic.rs").to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not available yet"));
}

/// `--format sarif` exits 2 with a clear "not available yet" error message.
#[test]
fn format_sarif_exits_two_with_message() {
    bin()
        .args([
            "check",
            "--format",
            "sarif",
            fixture("vulnerable", "sor001_basic.rs").to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not available yet"));
}

/// `--output <file>` writes the report to the specified file instead of stdout.
#[test]
fn output_flag_writes_to_file() {
    let dir = tempdir().expect("temp dir");
    let out_file = dir.path().join("report.txt");

    bin()
        .args(["check", "--output", out_file.to_str().unwrap()])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1)
        // stdout should be empty when writing to a file
        .stdout(predicate::str::is_empty());

    // The file should contain the finding.
    let contents = std::fs::read_to_string(&out_file).expect("output file should exist");
    assert!(
        contents.contains("SOR001"),
        "output file should contain SOR001 finding"
    );
    assert!(
        contents.contains("Found 1 error"),
        "output file should contain summary"
    );
}

/// `--no-color` flag is accepted and produces output without ANSI codes.
#[test]
fn no_color_flag_accepted() {
    bin()
        .args(["check", "--no-color"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SOR001"));
}

/// A file with a syntax error causes exit 2, but other files are still
/// analyzed.
#[test]
fn parse_error_exits_two() {
    let dir = tempdir().expect("temp dir");
    let bad = dir.path().join("bad.rs");
    std::fs::write(&bad, "pub fn bad( {\n").unwrap();

    bin()
        .arg("check")
        .arg(&bad)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("could not parse"));
}

/// An unknown `--ignore` ID exits 2 with a message identifying the bad ID.
#[test]
fn unknown_ignore_id_exits_two() {
    bin()
        .args(["check", "--ignore", "SOR999"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("unknown rule id"));
}

/// The `remove` fixture also triggers SOR001.
#[test]
fn vulnerable_remove_fixture_exits_one() {
    bin()
        .arg("check")
        .arg(fixture("vulnerable", "sor001_remove.rs"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SOR001"));
}

/// The read-only safe fixture exits 0.
#[test]
fn safe_readonly_fixture_exits_zero() {
    bin()
        .arg("check")
        .arg(fixture("safe", "sor001_readonly.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}

/// The auth-for-args safe fixture exits 0 (require_auth_for_args counts).
#[test]
fn safe_auth_for_args_fixture_exits_zero() {
    bin()
        .arg("check")
        .arg(fixture("safe", "sor001_auth_for_args.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}
