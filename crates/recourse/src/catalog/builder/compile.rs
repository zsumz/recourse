//! Compilation of validated merged registrations into deterministic artifacts.

use std::collections::BTreeMap;

use http::Uri;

use crate::catalog::{
    CatalogDiagnostic, CatalogIssue, Code, CodeNumber,
    artifact::{DiagnosticArtifactParts, HealthSurface, HttpSurface, OperationSurface},
};

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
    if type_uri.parse::<Uri>().is_err() {
        issues.push(CatalogIssue::InvalidTypeUri {
            number: registration.number,
            value: type_uri,
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
