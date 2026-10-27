//! Implementations of the CLI subcommands.
//!
//! The `check` command implements the full pipeline described in the README's
//! "How It Works" section:
//!
//! ```text
//! Discovery -> Parse -> ContractModel -> Rule engine -> Filter -> Render
//! ```
//!
//! # Configuration precedence (highest → lowest)
//!
//! | Layer | Source |
//! |---|---|
//! | 1 | Inline suppression comments (`// soroban-lint: allow(…)`) |
//! | 2 | CLI flags (`--select`, `--ignore`, `--fail-on`, `--min-severity`, …) |
//! | 3 | `soroban-lint.toml` (loaded here) |
//! | 4 | Built-in defaults |

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::Instant;

use lint_core::{
    apply_suppressions, find_config, load_config, scan_suppressions, ConfigError, ContractModel,
    DiscoveryOptions, Finding, LintConfig, ResolvedRuleConfig, RuleConfig, RuleContext,
    RuleRegistry, RuleStatus, Severity,
};
use lint_report::{
    json::JsonReporter, pretty::PrettyReporter, sarif::SarifReporter, Report, Reporter,
};
use lint_rules::builtin_registry;

use crate::cli::{CheckArgs, ExplainArgs, Format};
use crate::exit::{self, ExitCode};

/// Runs `soroban-lint check`.
pub fn check(args: CheckArgs) -> ExitCode {
    let registry = builtin_registry();

    // ── Validate --select / --ignore IDs ─────────────────────────────────
    let select_ids = match parse_and_validate_ids(&args.select, "--select", &registry) {
        Ok(ids) => ids,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::Error;
        }
    };
    let ignore_ids = match parse_and_validate_ids(&args.ignore, "--ignore", &registry) {
        Ok(ids) => ids,
        Err(msg) => {
            eprintln!("error: {msg}");
            return ExitCode::Error;
        }
    };

    // (all three formats are now implemented; no format guard needed)

    // ── Load configuration (layer 3) ──────────────────────────────────────
    let lint_config = match load_lint_config(args.config.as_deref(), &args.path) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::Error;
        }
    };

    // ── Resolve effective settings: CLI overrides config ──────────────────
    // Layer 2 (CLI) beats layer 3 (config file).
    let min_severity: Severity = args.min_severity.into();
    // fail_on: CLI flag beats config.
    let fail_on: Severity = {
        let cli_default: Severity = args.fail_on.into();
        // If the user explicitly passed --fail-on the clap default is "error";
        // we use CLI always since we have no way to distinguish "explicitly set"
        // from "default" in clap without extra tracking.  The config value is
        // used as a fallback only when we detect the CLI default matches the
        // built-in default (error).
        if cli_default == Severity::Error {
            lint_config.general.fail_on
        } else {
            cli_default
        }
    };
    let include_tests = args.include_tests || lint_config.general.include_tests;
    let use_color = color_enabled(args.no_color);

    // ── Discovery ─────────────────────────────────────────────────────────
    let start = Instant::now();
    let mut options = DiscoveryOptions::with_include_tests(include_tests);
    options.exclude = lint_config.general.exclude.clone();

    let discovery = lint_core::discover(&args.path, &options);

    let had_parse_errors = discovery.has_errors();
    for error in &discovery.errors {
        eprintln!(
            "error: could not parse `{}`: {} ({}:{})",
            error.path.display(),
            error.message,
            error.line,
            error.column,
        );
    }

    // ── Analysis ──────────────────────────────────────────────────────────
    let mut all_findings: Vec<Finding> = Vec::new();
    let files_scanned = discovery.files.len() + discovery.errors.len();

    for source_file in &discovery.files {
        let model = ContractModel::build(&source_file.ast);

        // Scan for inline suppression directives (layer 1).
        let supp_map = scan_suppressions(&source_file.source);

        // Emit warning-level findings for unknown suppression IDs.
        for diag in &supp_map.diagnostics {
            all_findings.push(Finding::new(
                "suppression",
                "unknown-suppression-id",
                Severity::Warning,
                &diag.message,
                &source_file.path,
                diag.line,
                1,
            ));
        }

        let mut file_findings: Vec<Finding> = Vec::new();

        for entry in registry.iter() {
            if entry.status() == RuleStatus::Planned {
                continue;
            }
            if !should_run_rule(entry.id(), &select_ids, &ignore_ids) {
                continue;
            }

            let resolved = lint_config.rule(entry.id());
            if !resolved.enabled {
                continue;
            }
            let rule_config = resolved_to_rule_config(&resolved);

            let ctx = RuleContext::new(
                &source_file.ast,
                &source_file.path,
                &source_file.source,
                &model,
                &rule_config,
            );

            let findings = entry.rule().check(&ctx);
            file_findings.extend(findings);
        }

        // Apply inline suppressions (layer 1).
        let file_findings = apply_suppressions(file_findings, &supp_map);

        // Apply --min-severity filter.
        let file_findings: Vec<Finding> = file_findings
            .into_iter()
            .filter(|f| f.severity >= min_severity)
            .collect();

        all_findings.extend(file_findings);
    }

    all_findings.sort_by(|a, b| {
        (&a.file, a.line, a.column, &a.rule_id).cmp(&(&b.file, b.line, b.column, &b.rule_id))
    });

    // ── Render ────────────────────────────────────────────────────────────
    let elapsed = start.elapsed();
    let report = Report {
        findings: all_findings,
        files_scanned,
        elapsed,
    };

    let rendered = match args.format {
        Format::Pretty => {
            let reporter = PrettyReporter::new(use_color);
            reporter.render(&report)
        }
        Format::Json => {
            let reporter = JsonReporter::new();
            reporter.render(&report)
        }
        Format::Sarif => {
            let reporter = SarifReporter::new();
            reporter.render(&report)
        }
    };

    let rendered = match rendered {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: failed to render report: {err}");
            return ExitCode::Error;
        }
    };

    if let Some(ref output_path) = args.output {
        if let Err(err) = write_file(output_path, &rendered) {
            eprintln!(
                "error: could not write output to `{}`: {err}",
                output_path.display()
            );
            return ExitCode::Error;
        }
    } else {
        print!("{rendered}");
        let _ = io::stdout().flush();
    }

    if had_parse_errors {
        return ExitCode::Error;
    }

    exit::code_for_findings(&report.findings, fail_on)
}

/// Runs `soroban-lint rules`.
pub fn rules() -> ExitCode {
    let registry = builtin_registry();
    println!(
        "{:<8}{:<30}{:<10}{:<18}STATUS",
        "ID", "NAME", "SEVERITY", "CATEGORY"
    );
    for entry in registry.iter() {
        println!(
            "{:<8}{:<30}{:<10}{:<18}{}",
            entry.id(),
            entry.name(),
            entry.default_severity(),
            entry.category(),
            entry.status().as_str(),
        );
    }
    ExitCode::Clean
}

/// Runs `soroban-lint explain <ID>`.
pub fn explain(args: ExplainArgs) -> ExitCode {
    let registry = builtin_registry();
    let normalized = args.id.to_ascii_uppercase();
    if registry.is_known_id(&normalized) {
        println!("Documentation for {normalized} is not documented yet.");
        ExitCode::Clean
    } else {
        eprintln!("error: unknown rule `{}`", args.id);
        ExitCode::Error
    }
}

/// Runs `soroban-lint init`.
///
/// Writes the default `soroban-lint.toml` to the current directory.
/// Refuses to overwrite an existing file (exit code 2).
pub fn init() -> ExitCode {
    let path = PathBuf::from("soroban-lint.toml");
    if path.exists() {
        eprintln!(
            "error: `{}` already exists; remove it first or edit it directly",
            path.display()
        );
        return ExitCode::Error;
    }
    match std::fs::write(&path, lint_core::DEFAULT_CONFIG_CONTENT) {
        Ok(()) => {
            println!("Created `{}`", path.display());
            ExitCode::Clean
        }
        Err(err) => {
            eprintln!("error: could not write `{}`: {err}", path.display());
            ExitCode::Error
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Loads the effective `LintConfig` for a run.
///
/// Priority: `--config` flag > auto-discovered `soroban-lint.toml` > defaults.
fn load_lint_config(
    config_arg: Option<&std::path::Path>,
    paths: &[PathBuf],
) -> Result<LintConfig, ConfigError> {
    if let Some(explicit) = config_arg {
        return load_config(explicit);
    }
    // Search upward from the first analysis path.
    let start = paths.first().and_then(|p| {
        if p.is_dir() {
            Some(p.as_path())
        } else {
            p.parent()
        }
    });
    if let Some(dir) = start {
        if let Some(found) = find_config(dir) {
            return load_config(&found);
        }
    }
    Ok(LintConfig::default())
}

/// Converts a `ResolvedRuleConfig` into the `RuleConfig` consumed by rules.
fn resolved_to_rule_config(resolved: &ResolvedRuleConfig) -> RuleConfig {
    RuleConfig {
        enabled: resolved.enabled,
        severity: resolved.severity,
        allow_function_names: resolved.allow_function_names.clone(),
        check_cargo_profile: resolved.check_cargo_profile,
        critical_keys: resolved.critical_keys.clone(),
        init_names: resolved.init_names.clone(),
    }
}

/// Parses and validates a comma-separated list of rule IDs.
fn parse_and_validate_ids(
    raw: &Option<String>,
    flag: &str,
    registry: &RuleRegistry,
) -> Result<Vec<String>, String> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let mut ids = Vec::new();
    for id in split_ids(raw) {
        let upper = id.to_ascii_uppercase();
        if !registry.is_known_id(&upper) {
            return Err(format!("unknown rule id `{id}` passed to {flag}"));
        }
        ids.push(upper);
    }
    Ok(ids)
}

fn should_run_rule(id: &str, select: &[String], ignore: &[String]) -> bool {
    if !select.is_empty() && !select.iter().any(|s| s == id) {
        return false;
    }
    if ignore.iter().any(|s| s == id) {
        return false;
    }
    true
}

fn split_ids(raw: &str) -> impl Iterator<Item = &str> {
    raw.split(',').map(str::trim).filter(|id| !id.is_empty())
}

fn color_enabled(no_color_flag: bool) -> bool {
    if no_color_flag || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

fn write_file(path: &PathBuf, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, content)
}
