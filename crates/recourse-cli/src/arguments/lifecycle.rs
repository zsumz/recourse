//! Check, accept, and reservation command parsing.

use std::ffi::OsString;

use recourse::catalog::Reservation;

use super::{ArgumentError, CatalogPaths, Command, OutputFormat, cursor::Cursor};

pub(super) fn parse_check(values: &[OsString]) -> Result<Command, ArgumentError> {
    parse_catalog_paths(values, false).map(|(paths, _)| Command::Check(paths))
}

pub(super) fn parse_accept(values: &[OsString]) -> Result<Command, ArgumentError> {
    let (paths, acknowledge_breaking) = parse_catalog_paths(values, true)?;
    Ok(Command::Accept {
        paths,
        acknowledge_breaking,
    })
}

pub(super) fn parse_reserve(values: &[OsString]) -> Result<Command, ArgumentError> {
    let mut cursor = Cursor::new(values);
    let mut lock = None;
    let mut reservation = None;
    let mut format = None;
    while let Some(option) = cursor.option()? {
        match option.as_str() {
            "--lock" => set_once(&mut lock, cursor.path("--lock")?, "--lock")?,
            "--number" => set_once(
                &mut reservation,
                Reservation::Exact(cursor.number()?),
                "--number",
            )?,
            "--format" => set_once(&mut format, cursor.format()?, "--format")?,
            _ => return Err(ArgumentError::UnknownOption(option)),
        }
    }
    Ok(Command::Reserve {
        lock: required(lock, "--lock")?,
        reservation: reservation.unwrap_or(Reservation::Next),
        format: format.unwrap_or(OutputFormat::Human),
    })
}

fn parse_catalog_paths(
    values: &[OsString],
    accepts_breaking: bool,
) -> Result<(CatalogPaths, bool), ArgumentError> {
    let mut cursor = Cursor::new(values);
    let mut current = None;
    let mut lock = None;
    let mut format = None;
    let mut acknowledged = None;
    while let Some(option) = cursor.option()? {
        match option.as_str() {
            "--current" => set_once(&mut current, cursor.path("--current")?, "--current")?,
            "--lock" => set_once(&mut lock, cursor.path("--lock")?, "--lock")?,
            "--format" => set_once(&mut format, cursor.format()?, "--format")?,
            "--acknowledge-breaking" if accepts_breaking => {
                set_once(&mut acknowledged, true, "--acknowledge-breaking")?;
            }
            _ => return Err(ArgumentError::UnknownOption(option)),
        }
    }
    Ok((
        CatalogPaths {
            current: required(current, "--current")?,
            lock: required(lock, "--lock")?,
            format: format.unwrap_or(OutputFormat::Human),
        },
        acknowledged.unwrap_or(false),
    ))
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
