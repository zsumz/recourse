//! Internal compiler input for one deterministic catalog diagnostic.

use serde_json::Value;

use crate::catalog::{Code, CodeNumber};

use super::{CatalogDiagnostic, DiagnosticSurfaces, HealthSurface, HttpSurface, OperationSurface};

pub(crate) struct DiagnosticArtifactParts {
    pub(crate) number: CodeNumber,
    pub(crate) code: Code,
    pub(crate) type_uri: String,
    pub(crate) title: &'static str,
    pub(crate) detail: &'static str,
    pub(crate) suggestions: &'static [&'static str],
    pub(crate) docs: &'static str,
    pub(crate) evidence_schema: Value,
    pub(crate) http: Option<HttpSurface>,
    pub(crate) operation: Option<OperationSurface>,
    pub(crate) health: Option<HealthSurface>,
}

impl From<DiagnosticArtifactParts> for CatalogDiagnostic {
    fn from(parts: DiagnosticArtifactParts) -> Self {
        Self {
            number: parts.number,
            code: parts.code,
            type_uri: parts.type_uri,
            title: parts.title.to_owned(),
            detail: parts.detail.to_owned(),
            suggestions: parts.suggestions.iter().map(ToString::to_string).collect(),
            documentation_markdown: normalize_markdown(parts.docs),
            evidence_schema: parts.evidence_schema,
            surfaces: DiagnosticSurfaces::new(parts.http, parts.operation, parts.health),
        }
    }
}

fn normalize_markdown(markdown: &str) -> String {
    markdown.replace("\r\n", "\n").replace('\r', "\n")
}
