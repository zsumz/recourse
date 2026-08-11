//! Escaped labeled output lines shared by every Dispatch renderer.
//!
//! Remote display text is data. Every value written here passes through
//! terminal escaping first, and nothing remote is ever used as a format
//! string, a hyperlink, or a path.

use recourse::{
    catalog::{CatalogDiagnostic, Code},
    client::{ProtocolIssue, escape_terminal},
};
use serde_json::{Map, Value};

use crate::RenderError;

pub(crate) const UNRECOGNIZED: &str = "<unrecognized>";

pub(crate) fn code_text(code: Option<&Code>) -> String {
    code.map_or_else(|| UNRECOGNIZED.to_owned(), ToString::to_string)
}

pub(crate) fn append_field(rendered: &mut String, label: &str, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    rendered.push_str(label);
    rendered.push_str(": ");
    rendered.push_str(&escape_terminal(value));
    rendered.push('\n');
}

pub(crate) fn append_object(
    rendered: &mut String,
    label: &str,
    object: Option<&Map<String, Value>>,
) -> Result<(), RenderError> {
    let Some(object) = object else {
        return Ok(());
    };
    let encoded = serde_json::to_string(object).map_err(RenderError::RawDocument)?;
    append_field(rendered, label, Some(&encoded));
    Ok(())
}

pub(crate) fn append_suggestions(rendered: &mut String, suggestions: &[String]) {
    for suggestion in suggestions {
        rendered.push_str("Suggestion: ");
        rendered.push_str(&escape_terminal(suggestion));
        rendered.push('\n');
    }
}

pub(crate) fn append_issues<'a>(
    rendered: &mut String,
    issues: impl IntoIterator<Item = &'a ProtocolIssue>,
) {
    for issue in issues {
        rendered.push_str("Protocol issue: ");
        append_issue(rendered, issue);
        rendered.push('\n');
    }
}

pub(crate) fn append_type_issue(
    rendered: &mut String,
    definition: &CatalogDiagnostic,
    type_uri: Option<&str>,
) {
    match type_uri {
        Some(value) if value == definition.type_uri() => {}
        Some(value) => {
            rendered.push_str("Protocol issue: type URI ");
            rendered.push_str(&escape_terminal(value));
            rendered.push_str(" differs from ");
            rendered.push_str(definition.type_uri());
            rendered.push('\n');
        }
        None => rendered.push_str("Protocol issue: known code omitted type URI\n"),
    }
}

pub(crate) fn append_raw(
    rendered: &mut String,
    raw: &Map<String, Value>,
) -> Result<(), RenderError> {
    let encoded = serde_json::to_string(raw).map_err(RenderError::RawDocument)?;
    rendered.push_str("Data: ");
    rendered.push_str(&escape_terminal(&encoded));
    rendered.push('\n');
    Ok(())
}

fn append_issue(rendered: &mut String, issue: &ProtocolIssue) {
    match issue {
        ProtocolIssue::MalformedCode => rendered.push_str("malformed code"),
        ProtocolIssue::MalformedOperationDiagnosticId => {
            rendered.push_str("malformed operation diagnostic ID");
        }
        ProtocolIssue::MalformedHealthFindingId => rendered.push_str("malformed health finding ID"),
        ProtocolIssue::InvalidHealthSeverity => rendered.push_str("invalid health severity"),
        ProtocolIssue::InvalidObservationTime => {
            rendered.push_str("invalid health observation time");
        }
        ProtocolIssue::InvalidBodyStatus => rendered.push_str("invalid body status"),
        ProtocolIssue::TransportStatusMismatch { transport, body } => {
            rendered.push_str("transport status ");
            rendered.push_str(&transport.as_u16().to_string());
            rendered.push_str(" differs from body status ");
            rendered.push_str(&body.as_u16().to_string());
        }
        ProtocolIssue::UnexpectedTypeForCode { expected, received } => {
            rendered.push_str("type URI ");
            rendered.push_str(
                &received
                    .as_deref()
                    .map_or_else(|| "<missing>".to_owned(), escape_terminal),
            );
            rendered.push_str(" differs from ");
            rendered.push_str(expected);
        }
        ProtocolIssue::CatalogStatusMismatch {
            expected,
            transport,
        } => {
            rendered.push_str("transport status ");
            rendered.push_str(&transport.as_u16().to_string());
            rendered.push_str(" differs from catalog status ");
            rendered.push_str(&expected.as_u16().to_string());
        }
        ProtocolIssue::MissingRequiredHeader { header } => {
            rendered.push_str("missing required header ");
            rendered.push_str(&escape_terminal(header));
        }
        ProtocolIssue::CodeNotRegisteredForHttp => {
            rendered.push_str("known code is not registered for HTTP");
        }
        _ => rendered.push_str("unrecognized protocol inconsistency"),
    }
}
