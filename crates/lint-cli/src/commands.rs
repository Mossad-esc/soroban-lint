//! Implementations of the CLI subcommands.

use std::io::IsTerminal;

use lint_core::{RuleRegistry, Severity};
use lint_rules::builtin_registry;

use crate::cli::{CheckArgs, ExplainArgs};
use crate::exit::{self, ExitCode};

/// Runs `soroban-lint check`.
///
/// Rule execution is not implemented yet, so this validates the arguments and
/// reports that there is nothing to do.
pub fn check(args: CheckArgs) -> ExitCode {
    let registry = builtin_registry();

    if let Err(message) = validate_selection(&args, &registry) {
        eprintln!("error: {message}");
        return ExitCode::Error;
    }

    let min_severity: Severity = args.min_severity.into();
    let fail_on: Severity = args.fail_on.into();
    let use_color = color_enabled(args.no_color);
    let _ = (
        min_severity,
        &args.format,
        &args.output,
        &args.config,
        use_color,
    );
    let _ = (&args.path, args.include_tests);

    println!("no rules implemented yet");
    exit::code_for_findings(&[], fail_on)
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

/// Validates `--select` and `--ignore` against the registry.
fn validate_selection(args: &CheckArgs, registry: &RuleRegistry) -> Result<(), String> {
    for (flag, value) in [("--select", &args.select), ("--ignore", &args.ignore)] {
        let Some(raw) = value else { continue };
        for id in split_ids(raw) {
            if !registry.is_known_id(&id.to_ascii_uppercase()) {
                return Err(format!("unknown rule id `{id}` passed to {flag}"));
            }
        }
    }
    Ok(())
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
