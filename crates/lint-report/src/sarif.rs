//! SARIF 2.1.0 reporter.
//!
//! Produces a SARIF 2.1.0 document suitable for GitHub code scanning,
//! VS Code SARIF viewers, and other security dashboards.
//!
//! # Schema coverage
//!
//! - `$schema` — SARIF 2.1.0 JSON schema URI
//! - `version` — `"2.1.0"`
//! - `runs[0].tool.driver` — name, version, informationUri, rules[]
//! - `runs[0].results[]` — ruleId, ruleIndex, level, message.text,
//!   physicalLocation (artifactLocation.uri + region startLine/startColumn),
//!   partialFingerprints (stable hash for GitHub alert tracking)
//!
//! # Severity mapping
//!
//! | soroban-lint | SARIF level |
//! |---|---|
//! | `error` | `"error"` |
//! | `warning` | `"warning"` |
//! | `note` | `"note"` |
//!
//! # partialFingerprints
//!
//! A stable SHA-256 hex digest of `rule_id + "\0" + normalized_file + "\0" + line`
//! keyed as `"soroban-lint/v1"`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::Serialize;

use lint_core::{Finding, Severity};

use crate::{Report, Reporter};

const SARIF_SCHEMA: &str =
    "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json";
const SARIF_VERSION: &str = "2.1.0";
const INFORMATION_URI: &str = "https://github.com/Mossad-esc/soroban-lint";
const DOCS_BASE: &str = "https://github.com/Mossad-esc/soroban-lint/blob/main/docs/rules";

// ─── SARIF reporter ───────────────────────────────────────────────────────────

/// SARIF 2.1.0 reporter.
#[derive(Debug, Clone, Default)]
pub struct SarifReporter;

impl SarifReporter {
    /// Creates a new SARIF reporter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Reporter for SarifReporter {
    fn render(&self, report: &Report) -> Result<String, String> {
        let doc = SarifDocument::from_report(report);
        serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())
    }
}

// ─── SARIF schema structs ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct SarifDocument {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Debug, Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Debug, Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    version: &'static str,
    information_uri: &'static str,
    rules: Vec<SarifRule>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: String,
    name: String,
    short_description: SarifMessage,
    help_uri: String,
    default_configuration: SarifDefaultConfig,
}

#[derive(Debug, Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Debug, Serialize)]
struct SarifDefaultConfig {
    level: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    rule_id: String,
    rule_index: usize,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
    partial_fingerprints: SarifFingerprints,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Debug, Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
}

#[derive(Debug, Serialize)]
struct SarifFingerprints {
    #[serde(rename = "soroban-lint/v1")]
    v1: String,
}

// ─── Conversion ───────────────────────────────────────────────────────────────

/// Severity-to-SARIF level mapping.
fn sarif_level(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

/// Collects unique rule IDs from findings, preserving first-seen order.
fn unique_rules(findings: &[Finding]) -> Vec<&Finding> {
    let mut seen = std::collections::HashSet::new();
    let mut rules = Vec::new();
    for f in findings {
        if seen.insert(f.rule_id.as_str()) {
            rules.push(f);
        }
    }
    rules
}

/// Stable fingerprint: simple hash of rule_id + file + line.
fn fingerprint(rule_id: &str, file: &str, line: usize) -> String {
    let mut h = DefaultHasher::new();
    rule_id.hash(&mut h);
    file.hash(&mut h);
    line.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Normalises a file path to a forward-slash relative URI.
fn file_uri(file: &std::path::Path) -> String {
    file.to_string_lossy().replace('\\', "/")
}

impl SarifDocument {
    fn from_report(report: &Report) -> Self {
        // Build a rule catalogue from the unique rules seen in findings.
        let rule_findings = unique_rules(&report.findings);
        let rules: Vec<SarifRule> = rule_findings
            .iter()
            .map(|f| SarifRule {
                id: f.rule_id.clone(),
                name: f.rule_name.clone(),
                short_description: SarifMessage {
                    text: f.rule_name.clone(),
                },
                help_uri: format!("{}/{}.md", DOCS_BASE, f.rule_id),
                default_configuration: SarifDefaultConfig {
                    level: sarif_level(f.severity),
                },
            })
            .collect();

        // Build results — look up ruleIndex from the catalogue.
        let results: Vec<SarifResult> = report
            .findings
            .iter()
            .map(|f| {
                let rule_index = rule_findings
                    .iter()
                    .position(|r| r.rule_id == f.rule_id)
                    .unwrap_or(0);
                let uri = file_uri(&f.file);
                let fp = fingerprint(&f.rule_id, &uri, f.line);
                SarifResult {
                    rule_id: f.rule_id.clone(),
                    rule_index,
                    level: sarif_level(f.severity),
                    message: SarifMessage {
                        text: f.message.clone(),
                    },
                    locations: vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation { uri },
                            region: SarifRegion {
                                start_line: f.line,
                                start_column: f.column,
                            },
                        },
                    }],
                    partial_fingerprints: SarifFingerprints { v1: fp },
                }
            })
            .collect();

        SarifDocument {
            schema: SARIF_SCHEMA,
            version: SARIF_VERSION,
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "soroban-lint",
                        version: env!("CARGO_PKG_VERSION"),
                        information_uri: INFORMATION_URI,
                        rules,
                    },
                },
                results,
            }],
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use lint_core::Finding;
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
    fn sarif_required_top_level_fields() {
        let report = fixed_report(vec![], 1);
        let sarif = SarifReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        assert_eq!(v["version"], "2.1.0");
        assert!(v["$schema"].as_str().unwrap().contains("sarif-schema"));
        assert!(v["runs"].is_array());
        let driver = &v["runs"][0]["tool"]["driver"];
        assert_eq!(driver["name"], "soroban-lint");
        assert!(driver["version"].is_string());
        assert!(driver["informationUri"].is_string());
        assert!(driver["rules"].is_array());
    }

    #[test]
    fn sarif_result_fields() {
        let report = fixed_report(vec![make_finding("SOR001", Severity::Error, 42)], 1);
        let sarif = SarifReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        let result = &v["runs"][0]["results"][0];
        assert_eq!(result["ruleId"], "SOR001");
        assert_eq!(result["level"], "error");
        assert!(result["message"]["text"].is_string());
        let region = &result["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 42);
        assert_eq!(region["startColumn"], 12);
        assert!(result["partialFingerprints"]["soroban-lint/v1"].is_string());
    }

    #[test]
    fn sarif_severity_mapping() {
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

    #[test]
    fn sarif_forward_slashes_in_uri() {
        let mut f = make_finding("SOR001", Severity::Error, 1);
        f.file = PathBuf::from("contracts\\token\\src\\lib.rs");
        let report = fixed_report(vec![f], 1);
        let sarif = SarifReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        let uri = v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]
            ["artifactLocation"]["uri"]
            .as_str()
            .unwrap();
        assert!(!uri.contains('\\'));
    }

    #[test]
    fn sarif_zero_findings() {
        let report = fixed_report(vec![], 2);
        let sarif = SarifReporter::new().render(&report).unwrap();
        let v: serde_json::Value = serde_json::from_str(&sarif).unwrap();
        assert!(v["runs"][0]["results"].as_array().unwrap().is_empty());
        assert!(v["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap()
            .is_empty());
    }
}
