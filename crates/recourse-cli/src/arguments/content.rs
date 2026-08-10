//! Documentation and single-diagnostic explanation command parsing.

use std::ffi::OsString;

use recourse::catalog::Code;

use super::{ArgumentError, CatalogPaths, Command, OutputFormat, cursor::Cursor};

pub(super) fn parse_docs(values: &[OsString]) -> Result<Command, ArgumentError> {
    let mut cursor = Cursor::new(values);
    let mut current = None;
    let mut lock = None;
    let mut out = None;
    let mut format = None;
    while let Some(option) = cursor.option()? {
        match option.as_str() {
            "--current" => set_once(&mut current, cursor.path("--current")?, "--current")?,
            "--lock" => set_once(&mut lock, cursor.path("--lock")?, "--lock")?,
            "--out" => set_once(&mut out, cursor.path("--out")?, "--out")?,
            "--format" => set_once(&mut format, cursor.format()?, "--format")?,
            _ => return Err(ArgumentError::UnknownOption(option)),
        }
    }
    Ok(Command::Docs {
        paths: CatalogPaths {
            current: required(current, "--current")?,
            lock: required(lock, "--lock")?,
            format: format.unwrap_or(OutputFormat::Human),
        },
        out: required(out, "--out")?,
    })
}

pub(super) fn parse_explain(values: &[OsString]) -> Result<Command, ArgumentError> {
    let mut cursor = Cursor::new(values);
    let mut current = None;
    let mut code = None;
    let mut format = None;
    while let Some(argument) = cursor.option()? {
        match argument.as_str() {
            "--current" => set_once(&mut current, cursor.path("--current")?, "--current")?,
            "--format" => set_once(&mut format, cursor.format()?, "--format")?,
            value if !value.starts_with('-') => {
                set_once(&mut code, parse_code(value)?, "<CODE>")?;
            }
            _ => return Err(ArgumentError::UnknownOption(argument)),
        }
    }
    Ok(Command::Explain {
        current: required(current, "--current")?,
        code: required(code, "<CODE>")?,
        format: format.unwrap_or(OutputFormat::Human),
    })
}

fn parse_code(value: &str) -> Result<Code, ArgumentError> {
    value
        .parse()
        .map_err(|_| ArgumentError::InvalidCode(value.to_owned()))
}

fn set_once<T>(target: &mut Option<T>, value: T, name: &'static str) -> Result<(), ArgumentError> {
    if target.replace(value).is_some() {
        Err(ArgumentError::DuplicateOption(name))
    } else {
        Ok(())
    }
}

fn required<T>(value: Option<T>, name: &'static str) -> Result<T, ArgumentError> {
    value.ok_or(ArgumentError::MissingOption(name))
}
