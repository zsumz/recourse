//! Public command grammar and parsed artifact-tool intent.

mod content;
mod cursor;
mod error;
mod lifecycle;

use std::{ffi::OsString, path::PathBuf};

use recourse::catalog::{Code, Reservation};

pub(crate) use error::ArgumentError;

pub(crate) const USAGE: &str = "Usage:\n  cargo recourse check --current <catalog.json> --lock <catalog.lock> [--format human|json]\n  cargo recourse accept --current <catalog.json> --lock <catalog.lock> [--acknowledge-breaking] [--format human|json]\n  cargo recourse reserve --lock <catalog.lock> [--number <positive integer>] [--format human|json]\n  cargo recourse docs --current <catalog.json> --lock <catalog.lock> --out <directory> [--format human|json]\n  cargo recourse explain --current <catalog.json> <CODE> [--format human|json]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Check(CatalogPaths),
    Accept {
        paths: CatalogPaths,
        acknowledge_breaking: bool,
    },
    Reserve {
        lock: PathBuf,
        reservation: Reservation,
        format: OutputFormat,
    },
    Docs {
        paths: CatalogPaths,
        out: PathBuf,
    },
    Explain {
        current: PathBuf,
        code: Code,
        format: OutputFormat,
    },
    Help,
    Version,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CatalogPaths {
    pub(crate) current: PathBuf,
    pub(crate) lock: PathBuf,
    pub(crate) format: OutputFormat,
}

pub(crate) fn parse<I>(arguments: I) -> Result<Command, ArgumentError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut values = arguments.into_iter().collect::<Vec<_>>();
    if values.first().is_some_and(|value| value == "recourse") {
        values.remove(0);
    }
    let Some(command) = values.first().and_then(|value| value.to_str()) else {
        return Err(ArgumentError::MissingCommand);
    };
    let tail = &values[1..];
    match command {
        "check" => lifecycle::parse_check(tail),
        "accept" => lifecycle::parse_accept(tail),
        "reserve" => lifecycle::parse_reserve(tail),
        "docs" => content::parse_docs(tail),
        "explain" => content::parse_explain(tail),
        "help" | "--help" | "-h" if tail.is_empty() => Ok(Command::Help),
        "--version" | "-V" if tail.is_empty() => Ok(Command::Version),
        value => Err(ArgumentError::UnknownCommand(value.to_owned())),
    }
}
