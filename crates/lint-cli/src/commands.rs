//! Implementations of the CLI subcommands.
//!
//! The `check` command implements the full pipeline described in the README's
//! "How It Works" section:
//!
//! ```text
//! Discovery -> Parse -> ContractModel -> Rule engine -> Filter -> Render
//! ```

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::Instant;

use lint_core::{
    ContractModel, DiscoveryOptions, Finding, RuleConfig, RuleContext, RuleRegistry, RuleStatus,
    Severity,
};
use lint_report::{pretty::PrettyReporter, Report, Reporter};
use lint_rules::builtin_registry;

use crate::cli::{CheckArgs, ExplainArgs, Format};
use crate::exit::{self, ExitCode};

/// Runs `soroban-lint check`.
///
/// Implements the full analysis pipeline: discovery, parsing, model building,
/// rule execution, filtering, and rendering.
pub fn check(args: CheckArgs) -> ExitCode {
    let registry = builtin_registry();

    // --- Validate --select / --ignore IDs against the registry. ---
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

    // --- Format guard: json/sarif are not implemented yet. ---
    match args.format {
        Format::Json | Format::Sarif => {
            eprintln!(
                "error: --format {} is not available yet (planned for Phase 2). \
                 Use --format pretty.",
                format_name(args.format)
            );
            return ExitCode::Error;
        }
        Format::Pretty => {}
    }

    let min_severity: Severity = args.min_severity.into();
    let fail_on: Severity = args.fail_on.into();
    let use_color = color_enabled(args.no_color);

    // --- Discovery ---
    let start = Instant::now();
    let options = DiscoveryOptions::with_include_tests(args.include_tests);
    let discovery = lint_core::discover(&args.path, &options);

    // Report parse errors and continue (exit code 2 is set at the end).
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

    // --- Analysis ---
    let mut all_findings: Vec<Finding> = Vec::new();
    let files_scanned = discovery.files.len() + discovery.errors.len();

    for source_file in &discovery.files {
        let model = ContractModel::build(&source_file.ast);
        let config = RuleConfig::default();

        for entry in registry.iter() {
            // Skip rules that are not yet implemented.
            if entry.status() == RuleStatus::Planned {
                continue;
            }

            // Apply --select / --ignore filtering.
            if !should_run_rule(entry.id(), &select_ids, &ignore_ids) {
                continue;
            }

            let ctx = RuleContext::new(
                &source_file.ast,
                &source_file.path,
                "", // source text not needed by current rules; add if needed
                &model,
                &config,
            );

            let mut findings = entry.rule().check(&ctx);

            // Apply --min-severity filter.
            findings.retain(|f| f.severity >= min_severity);

            all_findings.extend(findings);
        }
    }

    // Sort findings for deterministic output: (file, line, column, rule_id).
    all_findings.sort_by(|a, b| {
        (&a.file, a.line, a.column, &a.rule_id).cmp(&(&b.file, b.line, b.column, &b.rule_id))
    });

    // --- Render ---
    let elapsed = start.elapsed();
    let report = Report {
        findings: all_findings,
        files_scanned,
        elapsed,
    };

    let reporter = PrettyReporter::new(use_color);
    let rendered = match reporter.render(&report) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("error: failed to render report: {err}");
            return ExitCode::Error;
        }
    };

    // --- Write output ---
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

    // --- Exit code ---
    // Exit 2 if there were parse errors, regardless of findings.
    if had_parse_errors {
        return ExitCode::Error;
    }

    exit::code_for_findings(&report.findings, fail_on)
}

/// Runs `soroban-lint rules`, printing the rule catalog as a table.
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

/// Runs `soroban-lint init`. Writing the config file lands with configuration
/// loading; this is a placeholder.
pub fn init() -> ExitCode {
    println!("init: not implemented yet");
    ExitCode::Clean
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parses and validates a comma-separated list of rule IDs.
///
/// Returns `Err` with a human-readable message if any ID is unknown.
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

/// Returns `true` if the rule with `id` should run given `select` and `ignore`
/// lists.
fn should_run_rule(id: &str, select: &[String], ignore: &[String]) -> bool {
    if !select.is_empty() && !select.iter().any(|s| s == id) {
        return false;
    }
    if ignore.iter().any(|s| s == id) {
        return false;
    }
    true
}

/// Splits a comma-separated list of rule identifiers, ignoring empty entries.
fn split_ids(raw: &str) -> impl Iterator<Item = &str> {
    raw.split(',').map(str::trim).filter(|id| !id.is_empty())
}

/// Returns whether colored output should be used.
///
/// Color is disabled by `--no-color`, by the `NO_COLOR` environment variable,
/// and when stdout is not a terminal.
fn color_enabled(no_color_flag: bool) -> bool {
    if no_color_flag || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Returns the display name of a format variant.
fn format_name(format: Format) -> &'static str {
    match format {
        Format::Pretty => "pretty",
        Format::Json => "json",
        Format::Sarif => "sarif",
    }
}

/// Writes `content` to a file, creating parent directories as needed.
fn write_file(path: &PathBuf, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, content)
}
