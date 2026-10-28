//! Snapshot and structural tests for the SARIF 2.1.0 reporter.

use lint_core::{Finding, Severity};
use lint_report::{sarif::SarifReporter, Report, Reporter};
use std::path::PathBuf;
use std::time::Duration;

fn fixed_report(findings: Vec<Finding>, files: usize) -> Report {
    Report {
        findings,
        files_scanned: files,
        elapsed: Duration::from_millis(0),
        baselined_count: 0,
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
}

#[test]
fn sarif_zero_findings_snapshot() {
    let report = fixed_report(vec![], 2);
    let sarif = SarifReporter::new().render(&report).unwrap();
    insta::assert_snapshot!("sarif_zero_findings", sarif);
}

#[test]
fn sarif_one_finding_snapshot() {
    let report = fixed_report(vec![make_finding("SOR001", Severity::Error, 42)], 2);
    let sarif = SarifReporter::new().render(&report).unwrap();
    insta::assert_snapshot!("sarif_one_finding", sarif);
}

#[test]
fn sarif_required_fields_present() {
    let report = fixed_report(vec![make_finding("SOR001", Severity::Error, 10)], 1);
    let sarif = SarifReporter::new().render(&report).unwrap();
    let v: serde_json::Value = serde_json::from_str(&sarif).unwrap();

    // Top-level required fields.
    assert_eq!(v["version"], "2.1.0");
    assert!(v["$schema"].as_str().unwrap().contains("sarif-schema"));

    // Tool driver.
    let driver = &v["runs"][0]["tool"]["driver"];
    assert_eq!(driver["name"], "soroban-lint");
    assert!(driver["version"].is_string());
    assert!(driver["informationUri"].is_string());
    assert!(driver["rules"].is_array());

    // Result.
    let result = &v["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "SOR001");
    assert!(result["level"].is_string());
    assert!(result["message"]["text"].is_string());
    assert!(result["partialFingerprints"]["soroban-lint/v1"].is_string());
    let region = &result["locations"][0]["physicalLocation"]["region"];
    assert_eq!(region["startLine"], 10);
    assert_eq!(region["startColumn"], 12);
}

#[test]
fn sarif_severity_levels_mapped_correctly() {
    let report = fixed_report(
        vec![
            make_finding("SOR001", Severity::Error, 1),
            make_finding("SOR002", Severity::Warning, 2),
            make_finding("SOR003", Severity::Note, 3),
        ],
        1,
    );
    let sarif = SarifReporter::new().render(&report).unwrap();
    let v: serde_json::Value = serde_json::from_str(&sarif).unwrap();
    let results = v["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results[0]["level"], "error");
    assert_eq!(results[1]["level"], "warning");
    assert_eq!(results[2]["level"], "note");
}
