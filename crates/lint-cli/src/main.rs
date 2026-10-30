//! Command-line interface for `soroban-lint`.

mod cli;
mod commands;
mod docs;
mod exit;

use std::process::ExitCode as ProcessExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ProcessExitCode {
    // Install a panic hook so that any unexpected internal panic produces a
    // user-friendly "internal error" message and exits with code 2 instead of
    // printing a raw Rust backtrace.  This implements the README exit-code
    // contract: exit 2 for "internal errors", never a silent success, never
    // an unformatted panic.
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map_or_else(|| "unknown location".to_owned(), |l| l.to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("(no message)");
        eprintln!("error: soroban-lint encountered an internal error at {location}: {payload}");
        eprintln!("  Please file an issue at https://github.com/Mossad-esc/soroban-lint/issues");
        eprintln!("  and include the command you ran and any relevant input files.");
        // Exit code 2 per the README exit-code contract.
        std::process::exit(2);
    }));

    let cli = Cli::parse();

    let outcome = match cli.command {
        Command::Check(args) => commands::check(args),
        Command::Rules => commands::rules(),
        Command::Explain(args) => commands::explain(args),
        Command::Init => commands::init(),
    };

    ProcessExitCode::from(outcome.code() as u8)
}
