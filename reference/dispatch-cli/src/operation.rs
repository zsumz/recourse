//! Durable failure rendering for diagnostics recorded after acceptance.

use recourse::{
    catalog::{Catalog, CatalogDiagnostic, CatalogSpec},
    client::{Classification, ReceivedOperationDiagnostic, escape_terminal},
};

use crate::{
    RenderError,
    field::{
        UNRECOGNIZED, append_field, append_issues, append_object, append_raw, append_suggestions,
        append_type_issue, code_text,
    },
};

/// Renders one tolerant durable diagnostic reported by a worker.
///
/// A durable failure is not a failed request, so nothing here prints an HTTP
/// status: the envelope carries impact instead.
pub fn render_operation<C: CatalogSpec>(
    catalog: &Catalog<C>,
    diagnostic: &ReceivedOperationDiagnostic,
) -> Result<String, RenderError> {
    let mut rendered = match catalog.classify_operation(diagnostic) {
        Classification::Known(definition) => known_heading(definition, diagnostic),
        _ => unknown_heading(diagnostic),
    };
    append_field(&mut rendered, "Detail", diagnostic.detail());
    append_object(&mut rendered, "Evidence", diagnostic.evidence())?;
    append_object(&mut rendered, "Impact", diagnostic.impact())?;
    append_suggestions(&mut rendered, diagnostic.suggestions());
    append_issues(&mut rendered, diagnostic.protocol_issues());
    append_raw(&mut rendered, diagnostic.raw())?;
    Ok(rendered)
}

fn known_heading(
    definition: &CatalogDiagnostic,
    diagnostic: &ReceivedOperationDiagnostic,
) -> String {
    let mut rendered = format!(
        "{} — {}\nDiagnostic {}\n",
        definition.code(),
        escape_terminal(definition.title()),
        identity(diagnostic)
    );
    append_type_issue(&mut rendered, definition, diagnostic.type_uri());
    rendered
}

fn unknown_heading(diagnostic: &ReceivedOperationDiagnostic) -> String {
    let mut rendered = format!(
        "Unknown durable diagnostic {}\nDiagnostic {}\n",
        escape_terminal(&code_text(diagnostic.code())),
        identity(diagnostic)
    );
    append_field(&mut rendered, "Title", diagnostic.title());
    rendered
}

fn identity(diagnostic: &ReceivedOperationDiagnostic) -> String {
    diagnostic
        .id()
        .map_or_else(|| UNRECOGNIZED.to_owned(), ToString::to_string)
}
