//! Baseline support: record known findings and suppress them on future runs.
//!
//! A baseline file is a JSON object with a version and an array of fingerprint
//! strings. Each fingerprint is a stable, line-number-independent identifier:
//!
//! ```text
//! FNV64( rule_id + "\0" + relative_path + "\0" + message_normalized )
//! ```
//!
//! `relative_path` uses forward slashes.
//! `message_normalized` has runs of decimal digits replaced with `N`.
//!
//! The same fingerprint is used as SARIF `partialFingerprints`.
//!
//! # File format
//!
//! ```json
//! {
//!   "version": "1",
//!   "fingerprints": ["abc123...", "def456..."]
//! }
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::finding::Finding;

// ─── Fingerprint ─────────────────────────────────────────────────────────────

/// Computes a stable fingerprint for a finding.
///
/// Independent of line numbers so that adding code above a finding does not
/// invalidate the baseline entry.
#[must_use]
pub fn fingerprint(finding: &Finding, workspace_root: &Path) -> String {
    let rel_path = finding
        .file
        .strip_prefix(workspace_root)
        .unwrap_or(&finding.file)
        .to_string_lossy()
        .replace('\\', "/");

    let norm_message = normalize_message(&finding.message);

    let raw = format!("{}\x00{}\x00{}", finding.rule_id, rel_path, norm_message);
    format!("{:016x}", fnv64(raw.as_bytes()))
}

/// Replaces runs of decimal digits with `N`.
fn normalize_message(msg: &str) -> String {
    let mut result = String::with_capacity(msg.len());
    let mut in_run = false;
    for ch in msg.chars() {
        if ch.is_ascii_digit() {
            if !in_run {
                result.push('N');
                in_run = true;
            }
        } else {
            in_run = false;
            result.push(ch);
        }
    }
    result
}

/// FNV-1a 64-bit hash (public domain algorithm).
fn fnv64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// ─── Baseline file ────────────────────────────────────────────────────────────

/// Error from a baseline operation.
#[derive(Debug)]
pub struct BaselineError {
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for BaselineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

/// In-memory representation of a baseline file.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Baseline {
    /// Format version — always `"1"`.
    pub version: String,
    /// Known fingerprints.
    pub fingerprints: HashSet<String>,
}

impl Baseline {
    /// Creates an empty baseline with version `"1"`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: "1".to_owned(),
            fingerprints: HashSet::new(),
        }
    }

    /// Returns `true` when `fp` is present in the baseline.
    #[must_use]
    pub fn contains(&self, fp: &str) -> bool {
        self.fingerprints.contains(fp)
    }

    /// Adds a fingerprint to the baseline.
    pub fn insert(&mut self, fp: String) {
        self.fingerprints.insert(fp);
    }
}

/// Loads a baseline from `path`.
///
/// # Errors
///
/// Returns `BaselineError` when the file cannot be read or parsed.
pub fn load_baseline(path: &Path) -> Result<Baseline, BaselineError> {
    let text = std::fs::read_to_string(path).map_err(|e| BaselineError {
        path: path.to_path_buf(),
        message: format!("could not read baseline file: {e}"),
    })?;
    serde_json::from_str(&text).map_err(|e| BaselineError {
        path: path.to_path_buf(),
        message: format!("could not parse baseline JSON: {e}"),
    })
}

/// Saves a baseline to `path`, creating parent directories as needed.
///
/// # Errors
///
/// Returns `BaselineError` when the file cannot be written.
pub fn save_baseline(baseline: &Baseline, path: &Path) -> Result<(), BaselineError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| BaselineError {
                path: path.to_path_buf(),
                message: format!("could not create parent directory: {e}"),
            })?;
        }
    }
    let text = serde_json::to_string_pretty(baseline).map_err(|e| BaselineError {
        path: path.to_path_buf(),
        message: format!("could not serialise baseline: {e}"),
    })?;
    std::fs::write(path, text).map_err(|e| BaselineError {
        path: path.to_path_buf(),
        message: format!("could not write baseline file: {e}"),
    })
}

/// Builds a `Baseline` from a slice of findings.
#[must_use]
pub fn baseline_from_findings(findings: &[Finding], workspace_root: &Path) -> Baseline {
    let mut b = Baseline::new();
    for f in findings {
        b.insert(fingerprint(f, workspace_root));
    }
    b
}

/// Filters `findings`, removing those whose fingerprint appears in `baseline`.
///
/// Returns `(kept, suppressed_count)`.
#[must_use]
pub fn apply_baseline(
    findings: Vec<Finding>,
    baseline: &Baseline,
    workspace_root: &Path,
) -> (Vec<Finding>, usize) {
    let mut kept = Vec::new();
    let mut suppressed = 0usize;
    for f in findings {
        if baseline.contains(&fingerprint(&f, workspace_root)) {
            suppressed += 1;
        } else {
            kept.push(f);
        }
    }
    (kept, suppressed)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::severity::Severity;
    use tempfile::tempdir;

    fn dummy(rule_id: &str, file: &str, msg: &str) -> Finding {
        Finding::new(
            rule_id,
            "test-rule",
            Severity::Warning,
            msg,
            PathBuf::from(file),
            10,
            1,
        )
    }

    #[test]
    fn fingerprint_stable_across_line_shifts() {
        let root = Path::new("/ws");
        let f1 = Finding::new(
            "SOR001",
            "r",
            Severity::Error,
            "msg",
            "/ws/src/lib.rs",
            42,
            1,
        );
        let f2 = Finding::new(
            "SOR001",
            "r",
            Severity::Error,
            "msg",
            "/ws/src/lib.rs",
            99,
            1,
        );
        assert_eq!(fingerprint(&f1, root), fingerprint(&f2, root));
    }

    #[test]
    fn different_rules_differ() {
        let root = Path::new("/ws");
        assert_ne!(
            fingerprint(&dummy("SOR001", "/ws/f.rs", "m"), root),
            fingerprint(&dummy("SOR002", "/ws/f.rs", "m"), root),
        );
    }

    #[test]
    fn apply_drops_known_finding() {
        let root = Path::new("/ws");
        let f = dummy("SOR001", "/ws/f.rs", "m");
        let mut b = Baseline::new();
        b.insert(fingerprint(&f, root));
        let (kept, suppressed) = apply_baseline(vec![f], &b, root);
        assert_eq!(kept.len(), 0);
        assert_eq!(suppressed, 1);
    }

    #[test]
    fn apply_keeps_new_finding() {
        let root = Path::new("/ws");
        let old = dummy("SOR001", "/ws/f.rs", "old");
        let new_f = dummy("SOR002", "/ws/f.rs", "new");
        let mut b = Baseline::new();
        b.insert(fingerprint(&old, root));
        let (kept, suppressed) = apply_baseline(vec![old, new_f], &b, root);
        assert_eq!(kept.len(), 1);
        assert_eq!(suppressed, 1);
        assert_eq!(kept[0].rule_id, "SOR002");
    }

    #[test]
    fn save_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("baseline.json");
        let root = Path::new("/ws");
        let f = dummy("SOR001", "/ws/f.rs", "m");
        let b = baseline_from_findings(&[f], root);
        save_baseline(&b, &path).unwrap();
        let loaded = load_baseline(&path).unwrap();
        assert_eq!(loaded.version, "1");
        assert_eq!(loaded.fingerprints, b.fingerprints);
    }

    #[test]
    fn missing_file_is_error() {
        let err = load_baseline(Path::new("/no/such/file.json")).unwrap_err();
        assert!(err.message.contains("could not read"));
    }

    #[test]
    fn normalize_digits() {
        assert_eq!(normalize_message("line 42 col 7"), "line N col N");
        assert_eq!(normalize_message("no digits"), "no digits");
    }
}
