//! `soroban-lint.toml` loading and the resolved configuration model.
//!
//! # Schema
//!
//! ```toml
//! [general]
//! exclude = ["target/**", "**/tests/**"]
//! include_tests = false
//! fail_on = "error"
//!
//! [rules.SOR001]
//! enabled = true
//! severity = "error"
//!
//! [rules.SOR001.allow_functions]
//! names = ["bump_ttl"]
//!
//! [rules.SOR002]
//! enabled = true
//! severity = "warning"
//! check_cargo_profile = true
//!
//! [rules.SOR004]
//! critical_keys = ["admin", "balance"]
//!
//! [rules.SOR007]
//! init_names = ["initialize", "init"]
//! ```
//!
//! Unknown fields in the TOML produce a `ConfigError` (exit code 2).
//!
//! # Precedence (highest to lowest)
//!
//! 1. Inline suppression comments in source (`// soroban-lint: allow(…)`)
//! 2. CLI flags (`--select`, `--ignore`, `--fail-on`, `--min-severity`,
//!    `--include-tests`)
//! 3. Config file (`soroban-lint.toml`)
//! 4. Built-in defaults
//!
//! This module covers layers 3 and 4.  Layer 2 is applied in the CLI command
//! handler.  Layer 1 is applied after rule execution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::severity::Severity;

// ─── Raw TOML types ──────────────────────────────────────────────────────────

/// Raw `[general]` table.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawGeneral {
    /// Glob patterns (relative to the config file) to skip.
    pub exclude: Option<Vec<String>>,
    /// Also analyze `#[cfg(test)]` modules and `tests/` directories.
    pub include_tests: Option<bool>,
    /// Exit non-zero at or above this severity.
    pub fail_on: Option<String>,
}

/// Raw per-rule table (`[rules.SORxxx]`).
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawRuleConfig {
    pub enabled: Option<bool>,
    pub severity: Option<String>,
    // SOR002-specific
    pub check_cargo_profile: Option<bool>,
    // SOR004-specific
    pub critical_keys: Option<Vec<String>>,
    // SOR007-specific
    pub init_names: Option<Vec<String>>,
    // SOR001-specific
    pub allow_functions: Option<RawAllowFunctions>,
}

/// `[rules.SOR001.allow_functions]`
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawAllowFunctions {
    pub names: Option<Vec<String>>,
}

/// The top-level structure of `soroban-lint.toml`.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    pub general: Option<RawGeneral>,
    pub rules: Option<HashMap<String, RawRuleConfig>>,
}

// ─── Resolved config ─────────────────────────────────────────────────────────

/// The known valid rule IDs.
const KNOWN_RULE_IDS: &[&str] = &[
    "SOR001", "SOR002", "SOR003", "SOR004", "SOR005", "SOR006", "SOR007", "SOR008",
];

/// Resolved `[general]` settings.
#[derive(Debug, Clone)]
pub struct GeneralConfig {
    /// Glob patterns to exclude from discovery.
    pub exclude: Vec<String>,
    /// Whether to analyze test code.
    pub include_tests: bool,
    /// Fail severity threshold.
    pub fail_on: Severity,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            exclude: Vec::new(),
            include_tests: false,
            fail_on: Severity::Error,
        }
    }
}

/// Resolved per-rule configuration.
#[derive(Debug, Clone, Default)]
pub struct ResolvedRuleConfig {
    /// Whether the rule is enabled (default: `true`).
    pub enabled: bool,
    /// Severity override (`None` = use rule default).
    pub severity: Option<Severity>,
    /// SOR002: inspect `[profile.release]` in Cargo.toml.
    pub check_cargo_profile: Option<bool>,
    /// SOR004: key name patterns treated as critical.
    pub critical_keys: Option<Vec<String>>,
    /// SOR007: function name patterns considered initializers.
    pub init_names: Option<Vec<String>>,
    /// SOR001: functions that are intentionally permissionless.
    pub allow_function_names: Vec<String>,
}

impl ResolvedRuleConfig {
    fn default_enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }
}

/// The fully resolved configuration for one run.
#[derive(Debug, Clone, Default)]
pub struct LintConfig {
    /// Path to the config file that was loaded, if any.
    pub config_path: Option<PathBuf>,
    /// General settings.
    pub general: GeneralConfig,
    /// Per-rule settings keyed by upper-case rule ID.
    pub rules: HashMap<String, ResolvedRuleConfig>,
}

impl LintConfig {
    /// Returns the resolved config for `rule_id`, falling back to defaults.
    #[must_use]
    pub fn rule(&self, rule_id: &str) -> ResolvedRuleConfig {
        self.rules
            .get(rule_id)
            .cloned()
            .unwrap_or_else(ResolvedRuleConfig::default_enabled)
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

/// An error produced while loading or validating the config file.
#[derive(Debug, Clone)]
pub struct ConfigError {
    /// Path to the file that caused the error.
    pub path: PathBuf,
    /// Human-readable description.
    pub message: String,
}

impl ConfigError {
    fn new(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.message)
    }
}

// ─── Loading ─────────────────────────────────────────────────────────────────

/// Loads the config file at `path`.
///
/// # Errors
///
/// Returns a `ConfigError` when the file cannot be read, fails to parse as
/// TOML, contains unknown fields, unknown rule IDs, or invalid severity
/// strings.
pub fn load_config(path: &Path) -> Result<LintConfig, ConfigError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::new(path, format!("could not read config file: {e}")))?;

    let raw: RawConfig = toml::from_str(&source)
        .map_err(|e| ConfigError::new(path, format!("TOML parse error: {e}")))?;

    let mut config = LintConfig {
        config_path: Some(path.to_path_buf()),
        ..LintConfig::default()
    };

    // ── [general] ──────────────────────────────────────────────────────────
    if let Some(gen) = raw.general {
        if let Some(exclude) = gen.exclude {
            config.general.exclude = exclude;
        }
        if let Some(it) = gen.include_tests {
            config.general.include_tests = it;
        }
        if let Some(fo) = gen.fail_on {
            config.general.fail_on = fo.parse::<Severity>().map_err(|_| {
                ConfigError::new(
                    path,
                    format!(
                        "invalid value for `general.fail_on`: `{fo}`; \
                         expected one of `note`, `warning`, `error`"
                    ),
                )
            })?;
        }
    }

    // ── [rules.*] ──────────────────────────────────────────────────────────
    if let Some(rules) = raw.rules {
        for (id, raw_rule) in rules {
            let upper = id.to_ascii_uppercase();
            if !KNOWN_RULE_IDS.contains(&upper.as_str()) {
                return Err(ConfigError::new(
                    path,
                    format!(
                        "unknown rule ID `{id}` in `[rules]`; \
                         known IDs are {}",
                        KNOWN_RULE_IDS.join(", ")
                    ),
                ));
            }

            let mut resolved = ResolvedRuleConfig::default_enabled();

            if let Some(enabled) = raw_rule.enabled {
                resolved.enabled = enabled;
            }
            if let Some(sev_str) = raw_rule.severity {
                resolved.severity = Some(sev_str.parse::<Severity>().map_err(|_| {
                    ConfigError::new(
                        path,
                        format!(
                            "invalid severity `{sev_str}` for rule `{id}`; \
                             expected one of `note`, `warning`, `error`"
                        ),
                    )
                })?);
            }
            if let Some(v) = raw_rule.check_cargo_profile {
                resolved.check_cargo_profile = Some(v);
            }
            if let Some(v) = raw_rule.critical_keys {
                resolved.critical_keys = Some(v);
            }
            if let Some(v) = raw_rule.init_names {
                resolved.init_names = Some(v);
            }
            if let Some(af) = raw_rule.allow_functions {
                resolved.allow_function_names = af.names.unwrap_or_default();
            }

            config.rules.insert(upper, resolved);
        }
    }

    Ok(config)
}

/// Searches for `soroban-lint.toml` starting at `start_dir`, walking up to
/// the workspace root.
///
/// Returns the path of the first file found, or `None`.
#[must_use]
pub fn find_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current = Some(start_dir);
    while let Some(dir) = current {
        let candidate = dir.join("soroban-lint.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        // Stop at workspace / git root (presence of Cargo.toml or .git).
        let at_root = dir.join("Cargo.toml").is_file() || dir.join(".git").is_dir();
        if at_root {
            return None;
        }
        current = dir.parent();
    }
    None
}

/// The default content of `soroban-lint.toml`, matching the README example.
pub const DEFAULT_CONFIG_CONTENT: &str = r#"# soroban-lint.toml

[general]
# Paths to skip (glob patterns, relative to this file)
exclude = ["target/**", "**/tests/**", "**/test_snapshots/**"]
# Analyze #[cfg(test)] code
include_tests = false
# Exit non-zero at or above this severity: "note" | "warning" | "error"
fail_on = "error"

[rules.SOR001]
enabled = true
severity = "error"

[rules.SOR002]
enabled = true
severity = "warning"
# Also verify overflow-checks in Cargo.toml release profile
check_cargo_profile = true

[rules.SOR003]
enabled = true
severity = "warning"

[rules.SOR004]
enabled = true
severity = "error"
# Key names treated as critical (case-insensitive substring match)
critical_keys = ["admin", "owner", "balance", "allowance", "config", "supply"]

[rules.SOR005]
enabled = true
severity = "warning"

[rules.SOR006]
enabled = true
severity = "warning"

[rules.SOR007]
enabled = true
severity = "error"
# Function names considered initializers
init_names = ["initialize", "init", "setup"]

[rules.SOR008]
enabled = true
severity = "error"

# Functions that are intentionally permissionless and should not trigger SOR001
[rules.SOR001.allow_functions]
names = ["bump_ttl", "get_price"]
"#;

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_config(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join("soroban-lint.toml");
        std::fs::write(&path, content).unwrap();
        path
    }

    // ── Valid config ─────────────────────────────────────────────────────────

    #[test]
    fn loads_valid_full_config() {
        let dir = tempdir().unwrap();
        let path = write_config(
            dir.path(),
            r#"
[general]
exclude = ["target/**"]
include_tests = true
fail_on = "warning"

[rules.SOR001]
enabled = true
severity = "error"

[rules.SOR001.allow_functions]
names = ["bump_ttl"]

[rules.SOR002]
enabled = false
severity = "warning"
check_cargo_profile = false

[rules.SOR004]
critical_keys = ["admin", "owner"]

[rules.SOR007]
init_names = ["init", "setup"]
"#,
        );

        let config = load_config(&path).expect("should load");
        assert_eq!(config.general.exclude, vec!["target/**"]);
        assert!(config.general.include_tests);
        assert_eq!(config.general.fail_on, Severity::Warning);

        let r1 = config.rule("SOR001");
        assert!(r1.enabled);
        assert_eq!(r1.severity, Some(Severity::Error));
        assert_eq!(r1.allow_function_names, vec!["bump_ttl"]);

        let r2 = config.rule("SOR002");
        assert!(!r2.enabled);
        assert_eq!(r2.check_cargo_profile, Some(false));

        let r4 = config.rule("SOR004");
        assert_eq!(
            r4.critical_keys.as_deref(),
            Some(&["admin".to_owned(), "owner".to_owned()][..])
        );

        let r7 = config.rule("SOR007");
        assert_eq!(
            r7.init_names.as_deref(),
            Some(&["init".to_owned(), "setup".to_owned()][..])
        );
    }

    #[test]
    fn defaults_when_no_config() {
        let config = LintConfig::default();
        assert!(config.general.exclude.is_empty());
        assert!(!config.general.include_tests);
        assert_eq!(config.general.fail_on, Severity::Error);
        let r = config.rule("SOR001");
        assert!(r.enabled);
        assert!(r.severity.is_none());
    }

    // ── Error classes ────────────────────────────────────────────────────────

    #[test]
    fn unknown_field_in_general_is_error() {
        let dir = tempdir().unwrap();
        let path = write_config(dir.path(), "[general]\nunknown_key = true\n");
        let err = load_config(&path).unwrap_err();
        assert!(
            err.message.contains("unknown") || err.message.contains("TOML"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn unknown_rule_id_is_error() {
        let dir = tempdir().unwrap();
        let path = write_config(dir.path(), "[rules.SOR999]\nenabled = true\n");
        let err = load_config(&path).unwrap_err();
        assert!(err.message.contains("SOR999"), "message: {}", err.message);
    }

    #[test]
    fn invalid_severity_in_general_is_error() {
        let dir = tempdir().unwrap();
        let path = write_config(dir.path(), "[general]\nfail_on = \"fatal\"\n");
        let err = load_config(&path).unwrap_err();
        assert!(
            err.message.contains("fail_on") || err.message.contains("fatal"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn invalid_severity_in_rule_is_error() {
        let dir = tempdir().unwrap();
        let path = write_config(dir.path(), "[rules.SOR001]\nseverity = \"critical\"\n");
        let err = load_config(&path).unwrap_err();
        assert!(
            err.message.contains("critical") || err.message.contains("severity"),
            "message: {}",
            err.message
        );
    }

    #[test]
    fn unknown_rule_field_is_error() {
        let dir = tempdir().unwrap();
        let path = write_config(
            dir.path(),
            "[rules.SOR001]\nenabled = true\nmy_typo = false\n",
        );
        let err = load_config(&path).unwrap_err();
        // serde deny_unknown_fields reports the field name
        assert!(
            err.message.contains("my_typo") || err.message.contains("unknown"),
            "message: {}",
            err.message
        );
    }

    // ── Precedence ───────────────────────────────────────────────────────────
    //
    // Precedence is enforced by the CLI command handler (Commit 3 wiring),
    // which applies config → then CLI overrides → then suppressions.
    // Here we verify the config values are correctly loaded and that they can
    // be overridden by a subsequent function call (simulating CLI layer).
    #[test]
    fn config_severity_can_be_overridden() {
        let dir = tempdir().unwrap();
        let path = write_config(dir.path(), "[rules.SOR001]\nseverity = \"warning\"\n");
        let config = load_config(&path).expect("should load");

        let resolved = config.rule("SOR001");
        assert_eq!(resolved.severity, Some(Severity::Warning));

        // Simulate CLI override: caller picks a different severity.
        let cli_severity = Severity::Note;
        // In real code the CLI layer wins; we just assert it would differ.
        assert_ne!(cli_severity, resolved.severity.unwrap());
    }

    // ── Default content round-trip ────────────────────────────────────────────

    #[test]
    fn default_config_content_parses_cleanly() {
        let dir = tempdir().unwrap();
        let path = write_config(dir.path(), DEFAULT_CONFIG_CONTENT);
        let config = load_config(&path).expect("default config should parse");
        // Spot-check a few expected defaults.
        assert!(!config.general.include_tests);
        assert_eq!(config.general.fail_on, Severity::Error);
        assert!(config.rule("SOR001").enabled);
        assert!(config.rule("SOR002").enabled);
        assert_eq!(
            config.rule("SOR001").allow_function_names,
            vec!["bump_ttl", "get_price"]
        );
    }

    // ── allow_functions wiring ───────────────────────────────────────────────

    #[test]
    fn allow_functions_names_are_loaded() {
        let dir = tempdir().unwrap();
        let path = write_config(
            dir.path(),
            "[rules.SOR001.allow_functions]\nnames = [\"deposit\", \"bump_ttl\"]\n",
        );
        let config = load_config(&path).expect("should load");
        assert_eq!(
            config.rule("SOR001").allow_function_names,
            vec!["deposit", "bump_ttl"]
        );
    }
}
