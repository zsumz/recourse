//! Filesystem-facing catalog, lock, compatibility, and documentation commands.

mod arguments;
mod command;
mod documentation;
mod error;
mod files;
mod report;

#[cfg(test)]
mod arguments_test;
#[cfg(test)]
mod files_test;

use std::process::ExitCode;

fn main() -> ExitCode {
    match arguments::parse(std::env::args_os().skip(1)) {
        Ok(command) => command::execute(command),
        Err(error) => {
            eprintln!("error: {error}\n\n{}", arguments::USAGE);
            ExitCode::from(2)
        }
    }
}
