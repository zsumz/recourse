//! Compatibility checking, acceptance, and reservation execution.

use std::{io::Write, process::ExitCode};

use recourse::catalog::{AcceptanceError, AcceptanceMode, CatalogLock, Code};

use crate::{
    arguments::{CatalogPaths, OutputFormat},
    error::CommandError,
    files, report,
};

pub(super) fn check(paths: &CatalogPaths) -> Result<ExitCode, CommandError> {
    let current = files::read_artifact(&paths.current)?;
    let lock = files::read_lock(&paths.lock)?;
    let report = lock.check(&current);
    report::write_report(&report, paths.format)?;
    Ok(compatibility_exit(&report))
}

pub(super) fn accept(
    paths: &CatalogPaths,
    acknowledge_breaking: bool,
) -> Result<ExitCode, CommandError> {
    let current = files::read_artifact(&paths.current)?;
    let Some(mut lock) = files::read_optional_lock(&paths.lock)? else {
        let lock = CatalogLock::from_artifact(&current);
        files::write_lock(&paths.lock, &lock)?;
        report::write_report(&lock.check(&current), paths.format)?;
        return Ok(ExitCode::SUCCESS);
    };
    let mode = if acknowledge_breaking {
        AcceptanceMode::AcknowledgeBreaking
    } else {
        AcceptanceMode::CompatibleOnly
    };
    match lock.accept(&current, mode) {
        Ok(report) => {
            files::write_lock(&paths.lock, &lock)?;
            report::write_report(&report, paths.format)?;
            Ok(ExitCode::SUCCESS)
        }
        Err(
            AcceptanceError::Forbidden(report)
            | AcceptanceError::BreakingRequiresAcknowledgement(report),
        ) => {
            report::write_report(&report, paths.format)?;
            Ok(ExitCode::from(1))
        }
        Err(source) => Err(CommandError::Accept(source)),
    }
}

pub(super) fn reserve(
    path: &std::path::Path,
    reservation: recourse::catalog::Reservation,
    format: OutputFormat,
) -> Result<ExitCode, CommandError> {
    let mut lock = files::read_lock(path)?;
    let entry = lock
        .reserve(reservation)
        .map_err(|source| CommandError::Write {
            path: path.to_owned(),
            source: std::io::Error::other(source),
        })?;
    let code = entry.code().clone();
    let type_uri = entry.type_uri().to_owned();
    files::write_lock(path, &lock)?;
    write_reservation(format, &code, &type_uri)?;
    Ok(ExitCode::SUCCESS)
}

pub(super) fn retire(
    path: &std::path::Path,
    code: &Code,
    reason: &str,
    replacement: Option<&Code>,
    format: OutputFormat,
) -> Result<ExitCode, CommandError> {
    let mut lock = files::read_lock(path)?;
    lock.retire(code, reason, replacement.cloned())
        .map_err(CommandError::Retire)?;
    files::write_lock(path, &lock)?;
    write_retirement(format, code, reason, replacement)?;
    Ok(ExitCode::SUCCESS)
}

fn write_reservation(
    format: OutputFormat,
    code: &recourse::catalog::Code,
    type_uri: &str,
) -> Result<(), CommandError> {
    let mut output = std::io::stdout().lock();
    match format {
        OutputFormat::Human => writeln!(output, "{code}").map_err(CommandError::stdout),
        OutputFormat::Json => {
            let value = serde_json::json!({
                "code": code,
                "number": code.number(),
                "state": "reserved",
                "type": type_uri,
            });
            serde_json::to_writer(&mut output, &value).map_err(CommandError::Json)?;
            writeln!(output).map_err(CommandError::stdout)
        }
    }
}

fn write_retirement(
    format: OutputFormat,
    code: &Code,
    reason: &str,
    replacement: Option<&Code>,
) -> Result<(), CommandError> {
    let mut output = std::io::stdout().lock();
    match format {
        OutputFormat::Human => writeln!(output, "{code}").map_err(CommandError::stdout),
        OutputFormat::Json => {
            let value = serde_json::json!({
                "code": code,
                "state": "retired",
                "reason": reason,
                "replacement": replacement,
            });
            serde_json::to_writer(&mut output, &value).map_err(CommandError::Json)?;
            writeln!(output).map_err(CommandError::stdout)
        }
    }
}

fn compatibility_exit(report: &recourse::catalog::CompatibilityReport) -> ExitCode {
    if report.is_compatible() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
