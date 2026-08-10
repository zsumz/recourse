//! Bounded parsing and semantic validation for catalog artifacts.

mod error;

use std::{collections::BTreeSet, fmt::Display};

use http::{HeaderName, StatusCode, Uri};
use serde_json::Value;

use crate::{client::DecodeLimits, client::decode_object};

use super::{CatalogArtifact, CatalogDiagnostic};
use crate::catalog::{Code, CodeNumber, schema, valid_problem_set_id};

pub use error::ArtifactParseError;

const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn parse_artifact(body: &[u8]) -> Result<CatalogArtifact, ArtifactParseError> {
    let limits = DecodeLimits::default()
        .with_max_body_bytes(MAX_ARTIFACT_BYTES)
        .with_max_nesting_depth(64)
        .with_max_object_properties(16_384)
        .with_max_array_items(16_384)
        .with_max_string_bytes(512 * 1024);
    let object = decode_object(body, limits).map_err(ArtifactParseError::Decode)?;
    let artifact =
        serde_json::from_value(Value::Object(object)).map_err(ArtifactParseError::Structure)?;
    validate(&artifact)?;
    Ok(artifact)
}

fn validate(artifact: &CatalogArtifact) -> Result<(), ArtifactParseError> {
    if artifact.schema_version != 1 {
        return Err(ArtifactParseError::UnsupportedSchemaVersion {
            found: artifact.schema_version,
        });
    }
    validate_identity(artifact)?;
    validate_diagnostics(artifact)?;
    validate_problem_sets(artifact)
}

fn validate_identity(artifact: &CatalogArtifact) -> Result<(), ArtifactParseError> {
    let identity = &artifact.catalog;
    let name_valid = identity
        .name
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase())
        && !identity.name.ends_with('-')
        && !identity.name.contains("--")
        && identity
            .name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !name_valid {
        return invalid("catalog.name", "must be canonical lowercase kebab case");
    }
    Code::new(&identity.prefix, CodeNumber::new(1))
        .map_err(|error| invalid_value("catalog.prefix", error))?;
    let uri = identity
        .type_base
        .parse::<Uri>()
        .map_err(|error| invalid_value("catalog.type_base", error))?;
    if !valid_type_base(&identity.type_base, &uri) {
        return invalid("catalog.type_base", "must be an absolute URI ending in '/'");
    }
    Ok(())
}

fn valid_type_base(value: &str, uri: &Uri) -> bool {
    if !value.ends_with('/') {
        return false;
    }
    match uri.scheme_str() {
        Some("http" | "https") => uri.authority().is_some() && uri.path().starts_with('/'),
        Some(_) => !uri.path().is_empty(),
        None => false,
    }
}

fn validate_diagnostics(artifact: &CatalogArtifact) -> Result<(), ArtifactParseError> {
    for pair in artifact.diagnostics.windows(2) {
        if pair[0].number >= pair[1].number {
            return invalid("diagnostics", "numbers must be strictly increasing");
        }
    }
    for diagnostic in &artifact.diagnostics {
        validate_diagnostic(artifact, diagnostic)?;
    }
    Ok(())
}

fn validate_diagnostic(
    artifact: &CatalogArtifact,
    diagnostic: &CatalogDiagnostic,
) -> Result<(), ArtifactParseError> {
    let path = format!("diagnostics.{}", diagnostic.code);
    if diagnostic.code.prefix() != artifact.catalog.prefix
        || diagnostic.code.number() != diagnostic.number
    {
        return invalid(&path, "number and code must identify the catalog namespace");
    }
    if diagnostic.type_uri != format!("{}{}", artifact.catalog.type_base, diagnostic.code) {
        return invalid(
            &format!("{path}.type"),
            "must be derived from code and type base",
        );
    }
    if [
        diagnostic.title.as_str(),
        diagnostic.detail.as_str(),
        diagnostic.documentation_markdown.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return invalid(&path, "title, detail, and documentation must be nonempty");
    }
    if diagnostic
        .suggestions
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return invalid(&format!("{path}.suggestions"), "entries must be nonempty");
    }
    validate_schemas(diagnostic, &path)?;
    validate_surfaces(diagnostic, &path)
}

fn validate_schemas(diagnostic: &CatalogDiagnostic, path: &str) -> Result<(), ArtifactParseError> {
    schema::validate_artifact(&diagnostic.evidence_schema).map_err(|violation| {
        ArtifactParseError::Invalid {
            path: format!("{path}.evidence_schema{}", violation.path),
            reason: violation.reason,
        }
    })?;
    if let Some(impact) = diagnostic.impact_schema() {
        schema::validate_artifact(impact).map_err(|violation| ArtifactParseError::Invalid {
            path: format!("{path}.surfaces.operation.impact_schema{}", violation.path),
            reason: violation.reason,
        })?;
    }
    Ok(())
}

fn validate_surfaces(diagnostic: &CatalogDiagnostic, path: &str) -> Result<(), ArtifactParseError> {
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
    validate_headers(diagnostic.required_headers().unwrap_or_default(), path)
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

fn validate_problem_sets(artifact: &CatalogArtifact) -> Result<(), ArtifactParseError> {
    let diagnostics = artifact
        .diagnostics
        .iter()
        .filter(|value| value.surfaces.supports_http())
        .map(|value| &value.code)
        .collect::<BTreeSet<_>>();
    for (id, codes) in &artifact.problem_sets {
        if !valid_problem_set_id(id) {
            return invalid(&format!("problem_sets.{id}"), "operation ID is invalid");
        }
        if codes.windows(2).any(|pair| pair[0] >= pair[1]) {
            return invalid(
                &format!("problem_sets.{id}"),
                "codes must be sorted and unique",
            );
        }
        if let Some(code) = codes.iter().find(|code| !diagnostics.contains(code)) {
            return invalid(
                &format!("problem_sets.{id}"),
                &format!("{code} is not a registered HTTP diagnostic"),
            );
        }
    }
    Ok(())
}

fn invalid<T>(path: &str, reason: &str) -> Result<T, ArtifactParseError> {
    Err(ArtifactParseError::Invalid {
        path: path.to_owned(),
        reason: reason.to_owned(),
    })
}

fn invalid_value(path: &str, error: impl Display) -> ArtifactParseError {
    ArtifactParseError::Invalid {
        path: path.to_owned(),
        reason: error.to_string(),
    }
}
