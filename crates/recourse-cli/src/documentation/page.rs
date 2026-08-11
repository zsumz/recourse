//! Active and retired diagnostic Markdown pages.

use std::fmt::Write as _;

use recourse::catalog::{CatalogArtifact, CatalogDiagnostic};

use super::{ReplacementSummary, markdown, schema};

pub(super) fn active(
    diagnostic: &CatalogDiagnostic,
    artifact: &CatalogArtifact,
) -> Result<String, serde_json::Error> {
    let mut body = heading(diagnostic, "Active");
    push_surfaces(&mut body, diagnostic);
    body.push_str(&schema::section("Evidence", diagnostic.evidence_schema())?);
    if let Some(impact) = diagnostic.impact_schema() {
        body.push_str(&schema::section("Impact", impact)?);
    }
    push_operations(&mut body, diagnostic, artifact);
    push_guidance(&mut body, diagnostic);
    Ok(body)
}

pub(super) fn retired(
    diagnostic: &CatalogDiagnostic,
    reason: &str,
    replacement: Option<ReplacementSummary<'_>>,
) -> Result<String, serde_json::Error> {
    let mut body = heading(diagnostic, "Retired");
    body.push_str("## Retirement\n\n");
    body.push_str(&markdown::text(reason));
    body.push_str("\n\n");
    if let Some(replacement) = replacement {
        let direct = replacement.direct();
        let terminal = replacement.terminal();
        let _ = writeln!(body, "Replacement: `{direct}`");
        if terminal != direct {
            let _ = writeln!(body, "Terminal replacement: `{terminal}`");
        }
        body.push('\n');
    }
    push_surfaces(&mut body, diagnostic);
    body.push_str(&schema::section(
        "Historical evidence",
        diagnostic.evidence_schema(),
    )?);
    if let Some(impact) = diagnostic.impact_schema() {
        body.push_str(&schema::section("Historical impact", impact)?);
    }
    push_guidance(&mut body, diagnostic);
    Ok(body)
}

fn heading(diagnostic: &CatalogDiagnostic, state: &str) -> String {
    format!(
        "# {}: {}\n\n- State: **{state}**\n- Type: `{}`\n\n## Detail\n\n{}\n\n",
        diagnostic.code(),
        markdown::text(diagnostic.title()),
        diagnostic.type_uri(),
        markdown::text(diagnostic.detail())
    )
}

fn push_surfaces(body: &mut String, diagnostic: &CatalogDiagnostic) {
    body.push_str("## Surfaces\n\n");
    if let Some(status) = diagnostic.http_status() {
        let _ = write!(
            body,
            "### HTTP\n\n- Status: `{status}`\n- Policy: `{}`\n",
            diagnostic.http_policy().unwrap_or("unknown")
        );
        let headers = diagnostic.required_headers().unwrap_or_default();
        if headers.is_empty() {
            body.push_str("- Required headers: none\n\n");
        } else {
            let _ = write!(body, "- Required headers: `{}`\n\n", headers.join("`, `"));
        }
    }
    if diagnostic.impact_schema().is_some() {
        body.push_str("- Durable operation diagnostic\n");
    }
    if diagnostic.supports_health() {
        body.push_str("- Health finding\n");
    }
    if diagnostic.http_status().is_none()
        && diagnostic.impact_schema().is_none()
        && !diagnostic.supports_health()
    {
        body.push_str("_No envelope surfaces._\n");
    }
    body.push('\n');
}

fn push_operations(body: &mut String, diagnostic: &CatalogDiagnostic, artifact: &CatalogArtifact) {
    let operations = artifact
        .problem_sets()
        .iter()
        .filter(|(_, codes)| codes.contains(diagnostic.code()))
        .map(|(operation, _)| operation)
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return;
    }
    body.push_str("## Declared operations\n\n");
    for operation in operations {
        let _ = writeln!(body, "- `{operation}`");
    }
    body.push('\n');
}

fn push_guidance(body: &mut String, diagnostic: &CatalogDiagnostic) {
    if !diagnostic.suggestions().is_empty() {
        body.push_str("## Suggestions\n\n");
        for suggestion in diagnostic.suggestions() {
            let _ = writeln!(body, "- {}", markdown::text(suggestion));
        }
        body.push('\n');
    }
    if !diagnostic.documentation_markdown().trim().is_empty() {
        body.push_str("## Guidance\n\n");
        body.push_str(diagnostic.documentation_markdown().trim());
        body.push('\n');
    }
}
