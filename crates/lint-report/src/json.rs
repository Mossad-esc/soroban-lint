//! JSON reporter.
//!
//! Produces a machine-readable JSON report matching the README schema exactly:
//!
//! ```json
//! {
//!   "tool": "soroban-lint",
//!   "version": "0.1.0",
//!   "summary": { "errors": 1, "warnings": 1, "notes": 0, "files_scanned": 2 },
//!   "findings": [ … ]
//! }
//! ```
//!
//! Field order is stable (controlled by struct layout + `serde` rename
//! attributes, not by `HashMap` iteration).  Findings are assumed pre-sorted
//! by the caller.  File paths use forward slashes on all OSes.

use serde::Serialize;

use lint_core::Finding;

use crate::{Report, Reporter};

/// JSON reporter.  Construct with [`JsonReporter::new`].
#[derive(Debug, Clone, Default)]
pub struct JsonReporter;

impl JsonReporter {
    /// Creates a new JSON reporter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Reporter for JsonReporter {
    fn render(&self, report: &Report) -> Result<String, String> {
        let output = JsonOutput::from_report(report);
        serde_json::to_string_pretty(&output).map_err(|e| e.to_string())
    }
}

// ─── Schema structs ───────────────────────────────────────────────────────────

/// Top-level JSON output.
#[derive(Debug, Serialize)]
struct JsonOutput {
    #[serde(rename = "tool")]
    tool: &'static str,
    #[serde(rename = "version")]
    version: &'static str,
    #[serde(rename = "summary")]
    summary: JsonSummary,
    #[serde(rename = "findings")]
    findings: Vec<JsonFinding>,
}

/// Summary counts.
#[derive(Debug, Serialize)]
struct JsonSummary {
    #[serde(rename = "errors")]
    errors: usize,
    #[serde(rename = "warnings")]
    warnings: usize,
    #[serde(rename = "notes")]
    notes: usize,
    #[serde(rename = "files_scanned")]
    files_scanned: usize,
}

/// One finding in the JSON output.
#[derive(Debug, Serialize)]
struct JsonFinding {
    #[serde(rename = "rule")]
    rule: String,
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "severity")]
    severity: &'static str,
    #[serde(rename = "message")]
    message: String,
    #[serde(rename = "file")]
    file: String,
    #[serde(rename = "line")]
    line: usize,
    #[serde(rename = "column")]
    column: usize,
    #[serde(rename = "help")]
    #[serde(skip_serializing_if = "Option::is_none")]
    help: Option<String>,
}

impl JsonOutput {
    fn from_report(report: &Report) -> Self {
        let findings = report
            .findings
            .iter()
            .map(JsonFinding::from_finding)
            .collect();

        Self {
            tool: "soroban-lint",
            version: env!("CARGO_PKG_VERSION"),
            summary: JsonSummary {
                errors: report.error_count(),
                warnings: report.warning_count(),
                notes: report.note_count(),
                files_scanned: report.files_scanned,
            },
            findings,
        }
    }
}

impl JsonFinding {
    fn from_finding(f: &Finding) -> Self {
        Self {
            rule: f.rule_id.clone(),
            name: f.rule_name.clone(),
            severity: f.severity.as_str(),
            message: f.message.clone(),
            file: f.file.to_string_lossy().replace('\\', "/"),
            line: f.line,
            column: f.column,
            help: f.help.clone(),
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::{Finding, Severity};
    use std::path::PathBuf;
    use std::time::Duration;

    fn fixed_report(findings: Vec<Finding>, files: usize) -> Report {
        Report {
            findings,
            files_scanned: files,
            elapsed: Duration::from_millis(42),
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
        .with_help("call `from.require_auth()` before mutating state")
    }

    #[test]
    fn zero_findings_schema() {
        let report = fixed_report(vec![], 2);
        let json = JsonReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["tool"], "soroban-lint");
        assert!(v["version"].is_string());
        assert_eq!(v["summary"]["errors"], 0);
        assert_eq!(v["summary"]["warnings"], 0);
        assert_eq!(v["summary"]["notes"], 0);
        assert_eq!(v["summary"]["files_scanned"], 2);
        assert!(v["findings"].as_array().unwrap().is_empty());
    }

    #[test]
    fn one_finding_schema() {
        let report = fixed_report(vec![make_finding("SOR001", Severity::Error, 42)], 2);
        let json = JsonReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["summary"]["errors"], 1);
        let findings = v["findings"].as_array().unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["rule"], "SOR001");
        assert_eq!(findings[0]["name"], "missing-require-auth");
        assert_eq!(findings[0]["severity"], "error");
        assert_eq!(findings[0]["line"], 42);
        assert_eq!(findings[0]["column"], 12);
        assert!(findings[0]["file"]
            .as_str()
            .unwrap()
            .contains("contracts/token/src/lib.rs"));
        // Forward slashes on all OSes.
        assert!(!findings[0]["file"].as_str().unwrap().contains('\\'));
    }

    #[test]
    fn many_findings_sorted_order_preserved() {
        let findings = vec![
            make_finding("SOR001", Severity::Error, 10),
            make_finding("SOR002", Severity::Warning, 20),
            make_finding("SOR003", Severity::Warning, 30),
        ];
        let report = fixed_report(findings, 4);
        let json = JsonReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["summary"]["errors"], 1);
        assert_eq!(v["summary"]["warnings"], 2);
        let arr = v["findings"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["rule"], "SOR001");
        assert_eq!(arr[1]["rule"], "SOR002");
        assert_eq!(arr[2]["rule"], "SOR003");
    }

    #[test]
    fn help_omitted_when_none() {
        let f = Finding::new(
            "SOR001",
            "missing-require-auth",
            Severity::Error,
            "msg",
            PathBuf::from("f.rs"),
            1,
            1,
        );
        let report = fixed_report(vec![f], 1);
        let json = JsonReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["findings"][0].get("help").is_none() || v["findings"][0]["help"].is_null());
    }

    #[test]
    fn windows_path_uses_forward_slashes() {
        let mut f = make_finding("SOR001", Severity::Error, 1);
        // Simulate a Windows-style path.
        f.file = PathBuf::from("contracts\\token\\src\\lib.rs");
        let report = fixed_report(vec![f], 1);
        let json = JsonReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(!v["findings"][0]["file"].as_str().unwrap().contains('\\'));
    }
}
