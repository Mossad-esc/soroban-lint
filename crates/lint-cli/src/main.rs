//! Command-line interface for `soroban-lint`.

mod cli;
mod commands;
mod exit;

use std::process::ExitCode as ProcessExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ProcessExitCode {
    let cli = Cli::parse();

    let outcome = match cli.command {
        Command::Check(args) => commands::check(args),
        Command::Rules => commands::rules(),
        Command::Explain(args) => commands::explain(args),
        Command::Init => commands::init(),
    };

    ProcessExitCode::from(outcome.code() as u8)
}
