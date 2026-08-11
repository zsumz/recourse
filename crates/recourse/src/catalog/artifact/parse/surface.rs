//! HTTP and cross-surface invariants for parsed catalog artifacts.

use http::{HeaderName, StatusCode};

use crate::http::mandatory_headers;

use super::{ArtifactParseError, CatalogDiagnostic, invalid, invalid_value};

pub(super) fn validate(
    diagnostic: &CatalogDiagnostic,
    path: &str,
) -> Result<(), ArtifactParseError> {
    let surfaces = &diagnostic.surfaces;
    if !surfaces.supports_http() && !surfaces.supports_operation() && !surfaces.supports_health() {
        return invalid(
            &format!("{path}.surfaces"),
            "at least one surface is required",
        );
    }
    let Some(status) = diagnostic.http_status() else {
        return Ok(());
    };
    if !StatusCode::from_u16(status)
        .is_ok_and(|value| value.is_client_error() || value.is_server_error())
    {
        return invalid(
            &format!("{path}.surfaces.http.status"),
            "must be a 4xx or 5xx status",
        );
    }
    if diagnostic.http_policy().is_none_or(str::is_empty) {
        return invalid(&format!("{path}.surfaces.http.policy"), "must be nonempty");
    }
    let headers = diagnostic.required_headers().unwrap_or_default();
    validate_headers(headers, path)?;
    for required in mandatory_headers(status) {
        if !headers
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(required))
        {
            return invalid(
                &format!("{path}.surfaces.http.required_headers"),
                &format!("status {status} requires {required}"),
            );
        }
    }
    Ok(())
}

fn validate_headers(headers: &[String], path: &str) -> Result<(), ArtifactParseError> {
    let mut previous = None;
    for header in headers {
        header.parse::<HeaderName>().map_err(|error| {
            invalid_value(&format!("{path}.surfaces.http.required_headers"), error)
        })?;
        if previous.is_some_and(|value: &String| value >= header) {
            return invalid(
                &format!("{path}.surfaces.http.required_headers"),
                "must be sorted and unique",
            );
        }
        previous = Some(header);
    }
    Ok(())
}
