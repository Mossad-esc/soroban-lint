//! Command-line argument definitions for `soroban-lint`.
//!
//! The surface matches the README's "Usage" section exactly: the commands
//! `check`, `rules`, `explain`, and `init` (plus clap's built-in `help`), and
//! only the documented `check` options.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lint_core::Severity;

/// Output format for the `check` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Human-readable terminal diagnostics.
    Pretty,
    /// Machine-readable JSON.
    Json,
    /// SARIF 2.1.0 for code scanning.
    Sarif,
}

/// A severity level supplied on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SeverityArg {
    /// Style or best-practice suggestion.
    Note,
    /// Risky pattern that deserves review.
    Warning,
    /// Likely a real bug or vulnerability.
    Error,
}

impl From<SeverityArg> for Severity {
    fn from(value: SeverityArg) -> Self {
        match value {
            SeverityArg::Note => Self::Note,
            SeverityArg::Warning => Self::Warning,
            SeverityArg::Error => Self::Error,
        }
    }
}

/// `soroban-lint` command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "soroban-lint",
    version,
    about = "A static analyzer and linter for Soroban smart contracts",
    long_about = None,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Analyze contract source and report findings.
    Check(CheckArgs),
    /// List available rules.
    Rules,
    /// Show detailed documentation for a rule.
    Explain(ExplainArgs),
    /// Create a default soroban-lint.toml.
    Init,
}

/// Arguments for `check`.
#[derive(Debug, Args)]
pub struct CheckArgs {
    /// Files or directories to analyze.
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: Vec<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value = "pretty")]
    pub format: Format,

    /// Write the report to a file instead of stdout.
    #[arg(long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Path to a config file.
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Run only these rules (comma-separated).
    #[arg(long, value_name = "IDS")]
    pub select: Option<String>,

    /// Skip these rules (comma-separated).
    #[arg(long, value_name = "IDS")]
    pub ignore: Option<String>,

    /// Report only `note`, `warning`, or `error` and above.
    #[arg(long, value_name = "LVL", default_value = "note")]
    pub min_severity: SeverityArg,

    /// Exit non-zero at or above this severity.
    #[arg(long, value_name = "LVL", default_value = "error")]
    pub fail_on: SeverityArg,

    /// Disable colored output.
    #[arg(long)]
    pub no_color: bool,

    /// Also analyze `#[cfg(test)]` modules and `tests/`.
    #[arg(long)]
    pub include_tests: bool,

    /// Ignore findings already recorded in this baseline file.
    #[arg(long, value_name = "FILE")]
    pub baseline: Option<PathBuf>,

    /// Write current findings to the --baseline file and exit 0.
    #[arg(long)]
    pub update_baseline: bool,
}

/// Arguments for `explain`.
#[derive(Debug, Args)]
pub struct ExplainArgs {
    /// Rule identifier, for example `SOR001`.
    #[arg(value_name = "ID")]
    pub id: String,
}
