//! Private deserialization shapes converted into validated artifact domain values.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::catalog::{Code, CodeNumber};

use super::{CatalogArtifact, CatalogDiagnostic, CatalogIdentity, DiagnosticSurfaces};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogArtifactWire {
    schema_version: u32,
    catalog: CatalogIdentity,
    diagnostics: Vec<CatalogDiagnosticWire>,
    problem_sets: BTreeMap<String, Vec<Code>>,
}

impl CatalogArtifactWire {
    pub(super) fn into_domain(self) -> CatalogArtifact {
        CatalogArtifact {
            schema_version: self.schema_version,
            catalog: self.catalog,
            diagnostics: self
                .diagnostics
                .into_iter()
                .map(CatalogDiagnosticWire::into_domain)
                .collect(),
            problem_sets: self.problem_sets,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogDiagnosticWire {
    number: CodeNumber,
    code: Code,
    #[serde(rename = "type")]
    type_uri: String,
    title: String,
    detail: String,
    suggestions: Vec<String>,
    documentation_markdown: String,
    evidence_schema: Value,
    surfaces: DiagnosticSurfaces,
}

impl CatalogDiagnosticWire {
    pub(crate) fn into_domain(self) -> CatalogDiagnostic {
        CatalogDiagnostic {
            number: self.number,
            code: self.code,
            type_uri: self.type_uri,
            title: self.title,
            detail: self.detail,
            suggestions: self.suggestions,
            documentation_markdown: self.documentation_markdown,
            evidence_schema: self.evidence_schema,
            surfaces: self.surfaces,
        }
    }
}
