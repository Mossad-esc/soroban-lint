//! Snapshot tests for the JSON reporter.

use lint_core::{Finding, Severity};
use lint_report::{json::JsonReporter, Report, Reporter};
use std::path::PathBuf;
use std::time::Duration;

fn fixed_report(findings: Vec<Finding>, files: usize) -> Report {
    Report {
        findings,
        files_scanned: files,
        elapsed: Duration::from_millis(0),
    }
}

fn make_finding(rule_id: &str, sev: Severity, line: usize) -> Finding {
    Finding::new(
        rule_id,
        "missing-require-auth",
        sev,
        "entrypoint `transfer` writes to storage but never calls `require_auth`",
        PathBuf::from("contracts/token/src/lib.rs"),
        line,
        12,
    )
    .with_help("call `from.require_auth()` before mutating state")
    .with_docs_url("https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules/SOR001.md")
}

#[test]
fn json_zero_findings() {
    let report = fixed_report(vec![], 2);
    let json = JsonReporter::new().render(&report).unwrap();
    insta::assert_snapshot!("json_zero_findings", json);
    // Also parse and verify structure.
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["tool"], "soroban-lint");
    assert_eq!(v["summary"]["files_scanned"], 2);
    assert!(v["findings"].as_array().unwrap().is_empty());
}

#[test]
fn json_one_finding() {
    let report = fixed_report(vec![make_finding("SOR001", Severity::Error, 42)], 2);
    let json = JsonReporter::new().render(&report).unwrap();
    insta::assert_snapshot!("json_one_finding", json);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["summary"]["errors"], 1);
    assert_eq!(v["findings"][0]["rule"], "SOR001");
    assert_eq!(v["findings"][0]["severity"], "error");
    assert_eq!(v["findings"][0]["line"], 42);
}

#[test]
fn json_many_findings() {
    let findings = vec![
        make_finding("SOR001", Severity::Error, 10),
        make_finding("SOR002", Severity::Warning, 20),
        make_finding("SOR003", Severity::Warning, 30),
    ];
    let report = fixed_report(findings, 4);
    let json = JsonReporter::new().render(&report).unwrap();
    insta::assert_snapshot!("json_many_findings", json);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["summary"]["errors"], 1);
    assert_eq!(v["summary"]["warnings"], 2);
    assert_eq!(v["findings"].as_array().unwrap().len(), 3);
}
