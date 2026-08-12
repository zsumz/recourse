//! Bounded parsing and semantic validation for catalog artifacts.

mod error;
mod surface;

use std::{collections::BTreeSet, fmt::Display};

use serde_json::Value;

use crate::client::decode_object;
use crate::wire::WireLimits;

use super::{CatalogArtifact, CatalogDiagnostic, artifact_limits};
use crate::catalog::{Code, CodeNumber, schema, valid_problem_set_id, valid_type_base};

pub use error::ArtifactParseError;

pub(super) fn parse_artifact(body: &[u8]) -> Result<CatalogArtifact, ArtifactParseError> {
    let object = decode_object(body, artifact_limits()).map_err(ArtifactParseError::Decode)?;
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
    if !valid_type_base(&identity.type_base) {
        return invalid(
            "catalog.type_base",
            "must be an absolute query-free and fragment-free URI with a path ending in '/'",
        );
    }
    Ok(())
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
    if diagnostic.type_uri.len() > WireLimits::DEFAULT_MAX_STRING_BYTES {
        return invalid(
            &format!("{path}.type"),
            "exceeds the default diagnostic wire string limit",
        );
    }
    if let Some(violation) = crate::catalog::metadata::validate(
        &diagnostic.title,
        &diagnostic.detail,
        &diagnostic.suggestions,
        &diagnostic.documentation_markdown,
    )
    .into_iter()
    .next()
    {
        return invalid(&format!("{path}.{}", violation.field), &violation.reason);
    }
    validate_schemas(diagnostic, &path)?;
    surface::validate(diagnostic, &path)
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
