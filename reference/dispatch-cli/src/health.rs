//! Current service-state rendering for findings inside a health resource.

use recourse::{
    catalog::{Catalog, CatalogDiagnostic, CatalogSpec},
    client::{Classification, ReceivedHealthFinding, escape_terminal},
    health::HealthSeverity,
};

use crate::{
    RenderError,
    field::{
        UNRECOGNIZED, append_field, append_issues, append_object, append_raw, append_suggestions,
        append_type_issue, code_text,
    },
};

/// Renders one tolerant current health finding published by a service.
///
/// Severity and observation time replace the HTTP status a Problem carries;
/// a finding describes present state rather than a failed request.
pub fn render_health<C: CatalogSpec>(
    catalog: &Catalog<C>,
    finding: &ReceivedHealthFinding,
) -> Result<String, RenderError> {
    let mut rendered = match catalog.classify_health(finding) {
        Classification::Known(definition) => known_heading(definition, finding),
        _ => unknown_heading(finding),
    };
    append_field(&mut rendered, "Severity", Some(severity(finding)));
    append_field(
        &mut rendered,
        "Observed",
        finding
            .observed_at()
            .map(recourse::health::ObservationTime::as_str),
    );
    append_field(&mut rendered, "Detail", finding.detail());
    append_object(&mut rendered, "Evidence", finding.evidence())?;
    append_suggestions(&mut rendered, finding.suggestions());
    append_issues(&mut rendered, finding.protocol_issues());
    append_raw(&mut rendered, finding.raw())?;
    Ok(rendered)
}

fn known_heading(definition: &CatalogDiagnostic, finding: &ReceivedHealthFinding) -> String {
    let mut rendered = format!(
        "{} — {}\nFinding {}\n",
        definition.code(),
        escape_terminal(definition.title()),
        identity(finding)
    );
    append_type_issue(&mut rendered, definition, finding.type_uri());
    rendered
}

fn unknown_heading(finding: &ReceivedHealthFinding) -> String {
    let mut rendered = format!(
        "Unknown finding {}\nFinding {}\n",
        escape_terminal(&code_text(finding.code())),
        identity(finding)
    );
    append_field(&mut rendered, "Title", finding.title());
    rendered
}

const fn severity(finding: &ReceivedHealthFinding) -> &'static str {
    match finding.severity() {
        Some(HealthSeverity::Degraded) => "degraded",
        Some(HealthSeverity::Unhealthy) => "unhealthy",
        None => UNRECOGNIZED,
    }
}

fn identity(finding: &ReceivedHealthFinding) -> String {
    finding
        .id()
        .map_or_else(|| UNRECOGNIZED.to_owned(), ToString::to_string)
}
