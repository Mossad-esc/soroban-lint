//! Test harness for rule fixture tests.
//!
//! # Fixture naming convention
//!
//! Every fixture file lives under `tests/fixtures/{vulnerable,safe}/` at the
//! workspace root and is named `sor<NNN>_<case>.rs`, for example:
//!
//! - `tests/fixtures/vulnerable/sor001_basic.rs`   — a contract that MUST trigger SOR001
//! - `tests/fixtures/safe/sor001_authed.rs`         — a contract that must NOT trigger SOR001
//!
//! Each **implemented** rule must have at least one file in each directory.
//!
//! # Running a rule against a fixture
//!
//! Use [`run_rule_on_fixture`] directly or the [`assert_rule_snapshot!`] macro.
//!
//! Snapshots are deterministic: findings are sorted by `(file, line, column,
//! rule_id)` and file paths are made relative to the workspace root before
//! serialisation. The `timing` field is stripped from test output.

use std::path::{Path, PathBuf};

use lint_core::{ContractModel, Finding, Rule, RuleConfig, RuleContext};

/// Workspace root, resolved at compile time from the manifest directory of
/// *this* crate (`crates/lint-rules`).
pub fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `crates/lint-rules` at compile time.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent() // crates/
        .and_then(Path::parent) // workspace root
        .expect("workspace root should exist")
        .to_path_buf()
}

/// Resolves the path of a fixture file.
///
/// `subdir` is `"vulnerable"` or `"safe"`.
/// `filename` is the basename, e.g. `"sor001_basic.rs"`.
pub fn fixture_path(subdir: &str, filename: &str) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join(subdir)
        .join(filename)
}

/// A single finding serialized in a snapshot-stable form.
///
/// All paths are relative to the workspace root, so snapshots are portable.
#[derive(Debug, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotFinding {
    pub rule_id: String,
    pub rule_name: String,
    pub severity: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub help: Option<String>,
    pub docs_url: Option<String>,
}

impl SnapshotFinding {
    fn from_finding(finding: &Finding, root: &Path) -> Self {
        let rel = finding
            .file
            .strip_prefix(root)
            .unwrap_or(&finding.file)
            .to_string_lossy()
            .replace('\\', "/"); // normalise on Windows
        Self {
            rule_id: finding.rule_id.clone(),
            rule_name: finding.rule_name.clone(),
            severity: finding.severity.to_string(),
            message: finding.message.clone(),
            file: rel,
            line: finding.line,
            column: finding.column,
            help: finding.help.clone(),
            docs_url: finding.docs_url.clone(),
        }
    }
}

/// Parses a fixture file, builds a [`ContractModel`], runs `rule`, and returns
/// the findings sorted for snapshot stability.
///
/// # Panics
///
/// Panics if the fixture file cannot be read or parsed — fixture files are
/// expected to be valid Rust source.
pub fn run_rule_on_fixture(rule: &dyn Rule, subdir: &str, filename: &str) -> Vec<SnapshotFinding> {
    let path = fixture_path(subdir, filename);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read fixture {}: {err}", path.display()));
    let ast = syn::parse_file(&source)
        .unwrap_or_else(|err| panic!("failed to parse fixture {}: {err}", path.display()));
    let model = ContractModel::build(&ast);
    let config = RuleConfig::default();
    let ctx = RuleContext::new(&ast, &path, &source, &model, &config);

    let mut findings = rule.check(&ctx);
    // Sort for deterministic snapshots.
    findings.sort_by(|a, b| {
        (&a.file, a.line, a.column, &a.rule_id).cmp(&(&b.file, b.line, b.column, &b.rule_id))
    });

    let root = workspace_root();
    findings
        .iter()
        .map(|f| SnapshotFinding::from_finding(f, &root))
        .collect()
}

/// Asserts that running `rule` on `tests/fixtures/$subdir/$filename` matches a
/// stored insta snapshot.
///
/// The snapshot name is derived from the rule ID and fixture filename, so it
/// is unique across all rules.
///
/// ```ignore
/// assert_rule_snapshot!(sor001::Sor001, "SOR001", "vulnerable", "sor001_basic.rs");
/// ```
#[macro_export]
macro_rules! assert_rule_snapshot {
    ($rule:expr, $rule_id:expr, $subdir:expr, $filename:expr) => {{
        let findings = $crate::harness::run_rule_on_fixture(&$rule, $subdir, $filename);
        let snapshot_name = format!(
            "{}__{}_{}",
            $rule_id,
            $subdir,
            $filename.trim_end_matches(".rs")
        );
        insta::assert_yaml_snapshot!(snapshot_name, findings);
    }};
}
