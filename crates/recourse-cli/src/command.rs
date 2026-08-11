//! Dispatch from parsed intent into domain-owned artifact commands.

mod content;
mod lifecycle;

use std::process::ExitCode;

use crate::{
    arguments::{Command, USAGE},
    error::CommandError,
};

pub(crate) fn execute(command: Command) -> ExitCode {
    match run(command) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> Result<ExitCode, CommandError> {
    match command {
        Command::Check(paths) => lifecycle::check(&paths),
        Command::Accept {
            paths,
            acknowledge_breaking,
        } => lifecycle::accept(&paths, acknowledge_breaking),
        Command::Reserve {
            lock,
            reservation,
            format,
        } => lifecycle::reserve(&lock, reservation, format),
        Command::Retire {
            lock,
            code,
            reason,
            replacement,
            format,
        } => lifecycle::retire(&lock, &code, &reason, replacement.as_ref(), format),
        Command::Docs { paths, out } => content::docs(&paths, &out),
        Command::Explain {
            current,
            code,
            format,
        } => content::explain(&current, &code, format),
        Command::Help => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("cargo-recourse {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
    }
}
