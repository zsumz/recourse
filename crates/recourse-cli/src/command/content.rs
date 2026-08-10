//! Diagnostic explanation and accepted documentation generation.

use std::{io::Write, path::Path, process::ExitCode};

use recourse::catalog::Code;

use crate::{
    arguments::{CatalogPaths, OutputFormat},
    documentation::GeneratedDocumentation,
    error::CommandError,
    files, report,
};

pub(super) fn explain(
    current: &Path,
    code: &Code,
    format: OutputFormat,
) -> Result<ExitCode, CommandError> {
    let artifact = files::read_artifact(current)?;
    let diagnostic = artifact
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == code);
    let Some(diagnostic) = diagnostic else {
        write_unknown_code(code, format)?;
        return Ok(ExitCode::from(1));
    };
    let mut output = std::io::stdout().lock();
    match format {
        OutputFormat::Human => {
            let body = crate::documentation::explain(diagnostic, &artifact)?;
            write!(output, "{body}").map_err(CommandError::stdout)?;
        }
        OutputFormat::Json => {
            serde_json::to_writer(&mut output, diagnostic).map_err(CommandError::Json)?;
            writeln!(output).map_err(CommandError::stdout)?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn docs(paths: &CatalogPaths, out: &Path) -> Result<ExitCode, CommandError> {
    let artifact = files::read_artifact(&paths.current)?;
    let lock = files::read_lock(&paths.lock)?;
    let compatibility = lock.check(&artifact);
    if !compatibility.changes().is_empty() {
        report::write_report(&compatibility, paths.format)?;
        return Ok(ExitCode::from(1));
    }
    let documentation = GeneratedDocumentation::render(&artifact, &lock)?;
    files::write_documentation(out, &documentation)?;
    write_docs_result(out, &documentation, paths.format)?;
    Ok(ExitCode::SUCCESS)
}

fn write_unknown_code(code: &Code, format: OutputFormat) -> Result<(), CommandError> {
    let mut output = std::io::stdout().lock();
    match format {
        OutputFormat::Human => writeln!(
            output,
            "error[REC-CLI-001]: diagnostic {code} is not in the catalog"
        )
        .map_err(CommandError::stdout),
        OutputFormat::Json => {
            let value = serde_json::json!({
                "error": {
                    "id": "REC-CLI-001",
                    "code": code,
                    "reason": "diagnostic is not in the catalog",
                }
            });
            serde_json::to_writer(&mut output, &value).map_err(CommandError::Json)?;
            writeln!(output).map_err(CommandError::stdout)
        }
    }
}

fn write_docs_result(
    out: &Path,
    documentation: &GeneratedDocumentation,
    format: OutputFormat,
) -> Result<(), CommandError> {
    let mut output = std::io::stdout().lock();
    match format {
        OutputFormat::Human => writeln!(
            output,
            "generated {} pages in {}",
            documentation.pages().len(),
            out.display()
        )
        .map_err(CommandError::stdout),
        OutputFormat::Json => {
            let paths = documentation
                .pages()
                .keys()
                .filter_map(|path| path.to_str())
                .collect::<Vec<_>>();
            let value = serde_json::json!({ "generated": paths });
            serde_json::to_writer(&mut output, &value).map_err(CommandError::Json)?;
            writeln!(output).map_err(CommandError::stdout)
        }
    }
}
