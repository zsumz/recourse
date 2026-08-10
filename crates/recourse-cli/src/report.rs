//! Human and machine rendering for structured compatibility reports.

use std::io::{self, Write};

use recourse::catalog::{CompatibilityChange, CompatibilityReport, CompatibilitySeverity};

use crate::{arguments::OutputFormat, error::CommandError};

pub(crate) fn write_report(
    report: &CompatibilityReport,
    format: OutputFormat,
) -> Result<(), CommandError> {
    match format {
        OutputFormat::Human => write_human(report),
        OutputFormat::Json => write_json(report),
    }
}

fn write_human(report: &CompatibilityReport) -> Result<(), CommandError> {
    let mut output = io::stdout().lock();
    if report.changes().is_empty() {
        return writeln!(output, "catalog is compatible; no changes").map_err(CommandError::output);
    }
    for change in report.changes() {
        write_change(&mut output, change).map_err(CommandError::output)?;
    }
    Ok(())
}

fn write_change(output: &mut impl Write, change: &CompatibilityChange) -> io::Result<()> {
    let label = match change.severity() {
        CompatibilitySeverity::Compatible => "note",
        CompatibilitySeverity::Breaking | CompatibilitySeverity::Forbidden => "error",
    };
    writeln!(output, "{label}[{}]: {}", change.id(), change.reason())?;
    if let Some(code) = change.code() {
        writeln!(output, "  diagnostic  {code}")?;
    }
    writeln!(output, "  path        {}", change.path())?;
    writeln!(output, "  previous    {}", change.previous())?;
    writeln!(output, "  current     {}", change.current())?;
    writeln!(output, "\n{}\n", change.action())
}

fn write_json(report: &CompatibilityReport) -> Result<(), CommandError> {
    let value = serde_json::json!({
        "compatible": report.is_compatible(),
        "unchanged": report.changes().is_empty(),
        "has_breaking": report.has_breaking(),
        "has_forbidden": report.has_forbidden(),
        "changes": report.changes(),
    });
    let mut output = io::stdout().lock();
    serde_json::to_writer(&mut output, &value).map_err(CommandError::Json)?;
    writeln!(output).map_err(CommandError::output)
}

impl CommandError {
    fn output(source: io::Error) -> Self {
        Self::Write {
            path: "<stdout>".into(),
            source,
        }
    }
}
