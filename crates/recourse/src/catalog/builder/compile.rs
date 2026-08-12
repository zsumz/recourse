//! Compilation of validated merged registrations into deterministic artifacts.

use std::collections::BTreeMap;

use crate::catalog::{
    CatalogDiagnostic, CatalogIssue, Code, CodeNumber,
    artifact::{DiagnosticArtifactParts, HealthSurface, HttpSurface, OperationSurface},
    valid_type_uri,
};
use crate::wire::WireLimits;

use super::{registration::Registration, validation::ValidatedNamespace};

pub(super) fn compile_diagnostics(
    namespace: &ValidatedNamespace,
    registrations: BTreeMap<CodeNumber, Registration>,
    issues: &mut Vec<CatalogIssue>,
) -> Vec<CatalogDiagnostic> {
    registrations
        .into_values()
        .filter_map(|registration| compile_diagnostic(namespace, registration, issues))
        .collect()
}

fn compile_diagnostic(
    namespace: &ValidatedNamespace,
    registration: Registration,
    issues: &mut Vec<CatalogIssue>,
) -> Option<CatalogDiagnostic> {
    let code = Code::new(namespace.prefix, registration.number).ok()?;
    let type_uri = format!("{}{code}", namespace.type_base);
    if !valid_type_uri(&type_uri) {
        issues.push(CatalogIssue::InvalidTypeUri {
            number: registration.number,
            value: type_uri,
        });
        return None;
    }
    if type_uri.len() > WireLimits::DEFAULT_MAX_STRING_BYTES {
        issues.push(CatalogIssue::TypeUriTooLong {
            number: registration.number,
            maximum: WireLimits::DEFAULT_MAX_STRING_BYTES,
            actual: type_uri.len(),
        });
        return None;
    }
    let evidence_schema = registration.evidence_schema.ok()?;
    let http = registration
        .http
        .map(|surface| HttpSurface::new(surface.status, surface.policy, surface.required_headers));
    let operation = match registration.operation {
        Some(surface) => Some(OperationSurface::new(surface.impact_schema.ok()?)),
        None => None,
    };
    Some(
        DiagnosticArtifactParts {
            number: registration.number,
            code,
            type_uri,
            title: registration.title,
            detail: registration.detail,
            suggestions: registration.suggestions,
            docs: registration.docs,
            evidence_schema,
            http,
            operation,
            health: registration.health.then(HealthSurface::new),
        }
        .into(),
    )
}
