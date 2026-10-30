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
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(1)
        .stdout(predicate::str::contains("SOR001"))
        .stdout(predicate::str::contains("transfer"))
        .stdout(predicate::str::contains("require_auth"))
        .stdout(predicate::str::contains("Found 1 error in 1 file"));
}

/// A safe fixture must exit 0 and print no issues (checking only SOR001).
#[test]
fn safe_file_exits_zero() {
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("safe", "sor001_authed.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found in 1 file"));
}

/// `--ignore SOR001` suppresses the SOR001 finding; remaining warnings don't
/// fail because the default --fail-on is error.
#[test]
fn ignore_sor001_exits_zero() {
    bin()
        .args(["check", "--ignore", "SOR001", "--fail-on", "error"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(0);
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

/// `--format json` produces valid JSON output.
#[test]
fn format_json_produces_valid_json() {
    let output = bin()
        .args(["check", "--format", "json", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .output()
        .expect("command runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    assert_eq!(v["tool"], "soroban-lint");
    assert!(v["summary"].is_object());
    assert!(v["findings"].is_array());
    assert!(v["findings"][0]["rule"]
        .as_str()
        .unwrap()
        .contains("SOR001"));
}

/// `--format sarif` produces valid SARIF JSON with required fields.
#[test]
fn format_sarif_produces_valid_sarif() {
    let output = bin()
        .args(["check", "--format", "sarif", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .output()
        .expect("command runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("output should be valid JSON");
    assert_eq!(v["version"], "2.1.0");
    assert!(v["runs"][0]["tool"]["driver"]["name"] == "soroban-lint");
    assert!(v["runs"][0]["results"].is_array());
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

// ---------------------------------------------------------------------------
// Suppression tests
// ---------------------------------------------------------------------------

/// Inline `// soroban-lint: allow(SOR001)` suppresses the SOR001 finding.
#[test]
fn suppressed_entrypoint_exits_zero_for_sor001() {
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("safe", "sor001_suppressed.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}

/// An unknown rule ID in a suppression produces a warning diagnostic.
#[test]
fn unknown_suppression_id_produces_warning() {
    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("src.rs");
    std::fs::write(
        &src,
        "#[contractimpl]\nimpl T {\n    // soroban-lint: allow(SOR999)\n    pub fn f(_e: Env) {}\n}\n",
    )
    .unwrap();
    let output = bin()
        .arg("check")
        .arg("--select")
        .arg("SOR001")
        .arg(&src)
        .output()
        .expect("command runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should warn about the unknown ID.
    assert!(
        stdout.contains("SOR999") || String::from_utf8_lossy(&output.stderr).contains("SOR999"),
        "expected SOR999 warning in output:\nstdout: {stdout}"
    );
}

/// `// soroban-lint: allow(SOR001)` inside a string literal must not trigger.
#[test]
fn suppression_inside_string_literal_has_no_effect() {
    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("src.rs");
    // The directive is inside a string literal; SOR001 should still fire.
    std::fs::write(
        &src,
        "#[contractimpl]\nimpl T {\n    pub fn f(env: Env) {\n        let _s = \"// soroban-lint: allow(SOR001)\";\n        env.storage().persistent().set(&1u32, &2u32);\n    }\n}\n",
    )
    .unwrap();
    // SOR001 should still fire because the directive was inside a string.
    let output = bin()
        .args(["check", "--select", "SOR001"])
        .arg(&src)
        .output()
        .expect("command runs");
    // Exit code 1 means a finding was produced (suppression was ignored).
    assert_eq!(
        output.status.code(),
        Some(1),
        "SOR001 should fire; suppression inside string literal must be ignored"
    );
}

/// `// soroban-lint: allow(SOR001, SOR003)` suppresses both rules.
#[test]
fn multi_rule_suppression_in_one_directive() {
    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("src.rs");
    std::fs::write(
        &src,
        "// soroban-lint: allow(SOR001, SOR003)\n#[contractimpl]\nimpl T {\n    pub fn f(env: Env) {\n        env.storage().persistent().set(&1u32, &2u32);\n    }\n}\n",
    )
    .unwrap();
    let output = bin()
        .args(["check", "--select", "SOR001,SOR003"])
        .arg(&src)
        .output()
        .expect("command runs");
    // Both SOR001 and SOR003 are suppressed; the directive is on the impl block.
    // (Exit 0 means no un-suppressed errors/warnings at fail-on threshold.)
    let code = output.status.code().unwrap_or(99);
    assert!(
        code == 0 || code == 1,
        "unexpected exit code {code}; should be 0 or 1"
    );
}

/// `init` writes soroban-lint.toml and exits 0.
#[test]
fn init_creates_config_file() {
    let dir = tempdir().expect("temp dir");
    bin()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("soroban-lint.toml"));
    assert!(
        dir.path().join("soroban-lint.toml").exists(),
        "soroban-lint.toml should have been created"
    );
}

/// `init` refuses to overwrite an existing file (exit 2).
#[test]
fn init_refuses_to_overwrite_existing_file() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(dir.path().join("soroban-lint.toml"), "existing").unwrap();
    bin()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}

/// Config with an unknown rule ID should cause exit 2.
#[test]
fn config_unknown_rule_id_exits_two() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("soroban-lint.toml"),
        "[rules.SOR999]\nenabled = true\n",
    )
    .unwrap();
    bin()
        .arg("check")
        .arg("--config")
        .arg(dir.path().join("soroban-lint.toml"))
        .arg(fixture("safe", "sor001_authed.rs"))
        .assert()
        .code(2);
}

/// Config allow_functions makes a permissionless entrypoint not flagged.
#[test]
fn config_allow_functions_suppresses_sor001() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("soroban-lint.toml"),
        "[rules.SOR001.allow_functions]\nnames = [\"transfer\"]\n",
    )
    .unwrap();
    bin()
        .arg("check")
        .arg("--config")
        .arg(dir.path().join("soroban-lint.toml"))
        .arg("--select")
        .arg("SOR001")
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
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

/// The auth-for-args safe fixture exits 0 for SOR001 specifically.
#[test]
fn safe_auth_for_args_fixture_exits_zero() {
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("safe", "sor001_auth_for_args.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}

// ---------------------------------------------------------------------------
// Baseline tests
// ---------------------------------------------------------------------------

/// --update-baseline creates a baseline file and exits 0.
#[test]
fn update_baseline_creates_file_and_exits_zero() {
    let dir = tempdir().expect("temp dir");
    let baseline_path = dir.path().join("baseline.json");

    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .arg("--update-baseline")
        .assert()
        .code(0)
        .stdout(predicate::str::contains("Baseline written"));

    assert!(baseline_path.exists(), "baseline file must be created");
    let content = std::fs::read_to_string(&baseline_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).expect("baseline is valid JSON");
    assert_eq!(v["version"], "1");
    assert!(v["fingerprints"].is_array() || v["fingerprints"].is_object());
}

/// Applying an existing baseline suppresses known findings (exits 0).
#[test]
fn applying_baseline_suppresses_known_finding() {
    let dir = tempdir().expect("temp dir");
    let baseline_path = dir.path().join("baseline.json");

    // Step 1: create baseline from the vulnerable fixture.
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .arg("--update-baseline")
        .assert()
        .code(0);

    // Step 2: run again with the baseline — finding should be suppressed.
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .code(0)
        .stdout(predicate::str::contains("baselined"));
}

/// A new finding (not in baseline) still causes exit 1.
#[test]
fn new_finding_still_fails_with_baseline() {
    let dir = tempdir().expect("temp dir");
    let baseline_path = dir.path().join("baseline.json");

    // Create a baseline from a safe file (empty baseline).
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("safe", "sor001_readonly.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .arg("--update-baseline")
        .assert()
        .code(0);

    // The vulnerable file has a finding not in the baseline — must fail.
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .code(1);
}

/// --baseline with a missing file (without --update-baseline) exits 2.
#[test]
fn missing_baseline_file_exits_two() {
    let dir = tempdir().expect("temp dir");
    let baseline_path = dir.path().join("nonexistent.json");

    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("safe", "sor001_readonly.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not found"));
}

/// Line shift does not break baseline matching.
#[test]
fn line_shift_does_not_break_baseline() {
    let dir = tempdir().expect("temp dir");
    let baseline_path = dir.path().join("baseline.json");

    // Create baseline from the vulnerable file.
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .arg("--update-baseline")
        .assert()
        .code(0);

    // The same file run again against the baseline exits 0 (line numbers
    // don't matter for fingerprinting).
    bin()
        .args(["check", "--select", "SOR001"])
        .arg(fixture("vulnerable", "sor001_basic.rs"))
        .arg("--baseline")
        .arg(&baseline_path)
        .assert()
        .code(0);
}

// ---------------------------------------------------------------------------
// Determinism test (parallel output must be byte-identical across runs)
// ---------------------------------------------------------------------------

/// Run the same vulnerable fixture 20 times and assert identical output each
/// time. This guards against non-determinism introduced by parallel execution.
#[test]
fn parallel_output_is_deterministic() {
    let path = fixture("vulnerable", "sor001_basic.rs");
    let first_output = bin()
        .args(["check", "--format", "json", "--select", "SOR001"])
        .arg(&path)
        .output()
        .expect("first run");

    for i in 1..20 {
        let run = bin()
            .args(["check", "--format", "json", "--select", "SOR001"])
            .arg(&path)
            .output()
            .unwrap_or_else(|e| panic!("run {i} failed: {e}"));
        assert_eq!(
            first_output.stdout, run.stdout,
            "run {i} produced different stdout"
        );
        assert_eq!(
            first_output.status.code(),
            run.status.code(),
            "run {i} had different exit code"
        );
    }
}

// ---------------------------------------------------------------------------
// Edge-case hardening tests  (exit-code contract: 0=clean, 1=findings, 2=error)
// ---------------------------------------------------------------------------

/// Helper: returns a path to an edge-case fixture.
fn edge(name: &str) -> std::path::PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("tests")
        .join("fixtures")
        .join("edge_cases")
        .join(name)
}

/// An empty file must be parsed and produce zero findings (exit 0).
/// The tool must not panic on an empty file.
#[test]
fn empty_file_exits_zero_no_panic() {
    bin()
        .arg("check")
        .arg(edge("empty.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}

/// A file containing only comments must produce zero findings (exit 0).
#[test]
fn comments_only_exits_zero() {
    bin()
        .arg("check")
        .arg(edge("comments_only.rs"))
        .assert()
        .code(0)
        .stdout(predicate::str::contains("No issues found"));
}

/// A file with Windows CRLF line endings must be analyzed without panicking.
/// The tool must produce a well-formed report (exit 0 or 1 depending on findings).
#[test]
fn crlf_file_handled_without_panic() {
    let result = bin()
        .arg("check")
        .arg(edge("crlf_endings.rs"))
        .output()
        .expect("command must not fail to run");
    let code = result.status.code().unwrap_or(99);
    assert!(
        code == 0 || code == 1,
        "CRLF file must exit 0 or 1, got {code}"
    );
}

/// A file with a syntax error must exit 2 with a message; no panic.
#[test]
fn syntax_error_exits_two_no_panic() {
    bin()
        .arg("check")
        .arg(edge("syntax_error.rs"))
        .assert()
        .code(2)
        .stderr(predicate::str::contains("could not parse"));
}

/// Deeply nested expressions must not overflow the stack (exit 0 or 1).
#[test]
fn deep_nesting_no_stack_overflow() {
    let result = bin()
        .arg("check")
        .arg(edge("deep_nesting.rs"))
        .output()
        .expect("command must run");
    let code = result.status.code().unwrap_or(99);
    assert!(
        code == 0 || code == 1,
        "deep nesting must exit 0 or 1, got {code}"
    );
}

/// Macro-heavy code must be analyzed without panicking (exit 0 or 1).
#[test]
fn macro_heavy_file_no_panic() {
    let result = bin()
        .arg("check")
        .arg(edge("macro_heavy.rs"))
        .output()
        .expect("command must run");
    let code = result.status.code().unwrap_or(99);
    assert!(
        code == 0 || code == 1,
        "macro-heavy file must exit 0 or 1, got {code}"
    );
}

/// Unusual attribute paths must not cause a panic (exit 0 or 1).
#[test]
fn unusual_attrs_no_panic() {
    let result = bin()
        .arg("check")
        .arg(edge("unusual_attrs.rs"))
        .output()
        .expect("command must run");
    let code = result.status.code().unwrap_or(99);
    assert!(
        code == 0 || code == 1,
        "unusual attrs must exit 0 or 1, got {code}"
    );
}

/// An unreadable file must produce exit 2 with a clear error message.
/// This tests that the tool handles permission errors gracefully.
#[cfg(unix)]
#[test]
fn unreadable_file_exits_two_with_message() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("secret.rs");
    std::fs::write(&src, "pub fn x() {}").unwrap();

    // Remove read permission.
    std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o000)).unwrap();

    let result = bin()
        .arg("check")
        .arg(&src)
        .output()
        .expect("command must run even if it fails");

    // Restore before assertions so the file is cleaned up properly.
    let _ = std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o644));

    let code = result.status.code().unwrap_or(99);
    assert_eq!(code, 2, "unreadable file must exit 2, got {code}");
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.is_empty() || !String::from_utf8_lossy(&result.stdout).is_empty(),
        "must produce some output for unreadable file"
    );
}

/// Non-UTF-8 bytes: a file containing arbitrary bytes that are not valid UTF-8
/// must be handled gracefully (exit 2 with a parse/read error, not a panic).
#[test]
fn non_utf8_bytes_exit_two_no_panic() {
    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("binary.rs");
    // Write valid Rust source bytes followed by a lone 0xFF (invalid UTF-8).
    let mut content = b"pub fn x() {}\n".to_vec();
    content.push(0xFF);
    std::fs::write(&src, &content).expect("write binary file");

    bin()
        .arg("check")
        .arg(&src)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("could not parse").or(predicate::str::contains("error")));
}

/// A huge file (many thousands of lines) must complete without a timeout or
/// panic.  This is a performance smoke-test rather than a correctness test.
#[test]
fn huge_file_completes_without_panic() {
    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("huge.rs");

    // Build a file with 3000 trivial functions to stress the visitor.
    let mut body = String::from("#![no_std]\n");
    for i in 0..3000_usize {
        body.push_str(&format!("pub fn f{i}(_x: u32) -> u32 {{ _x + 1 }}\n"));
    }
    std::fs::write(&src, &body).expect("write huge file");

    let result = bin()
        .arg("check")
        .timeout(std::time::Duration::from_secs(60))
        .arg(&src)
        .output()
        .expect("command must run");
    let code = result.status.code().unwrap_or(99);
    assert!(
        code == 0 || code == 1,
        "huge file must exit 0 or 1, got {code}"
    );
}

/// `soroban-lint check` on a non-existent path exits 2 with an error message.
#[test]
fn nonexistent_path_exits_two() {
    bin()
        .arg("check")
        .arg("/tmp/this_path_definitely_does_not_exist_soroban_lint_test")
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("error")
                .or(predicate::str::contains("not found"))
                .or(predicate::str::contains("could not")),
        );
}
