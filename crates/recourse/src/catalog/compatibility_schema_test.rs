//! Conservative evidence and impact schema compatibility fixtures.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogArtifact, CatalogLock, CatalogSpec, CodeNumber};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum Diagnostic {}

impl DiagnosticType for Diagnostic {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1001);
    const TITLE: &'static str = "Schema fixture";
    const DETAIL: &'static str = "Schema compatibility fixture.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Schema compatibility fixture.";
}

impl HttpProblemType for Diagnostic {
    type Policy = Fixed<400>;
}

fn artifact(schema: serde_json::Value) -> CatalogArtifact {
    let baseline = Catalog::<DispatchCatalog>::builder()
        .problem::<Diagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact();
    let mut value = serde_json::to_value(baseline)
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"));
    value["diagnostics"][0]["evidence_schema"] = schema;
    parse(&value)
}

fn parse(value: &serde_json::Value) -> CatalogArtifact {
    let body = serde_json::to_vec(value)
        .unwrap_or_else(|error| panic!("schema fixture must encode: {error}"));
    CatalogArtifact::from_slice(&body)
        .unwrap_or_else(|error| panic!("schema fixture must remain valid: {error}"))
}

#[test]
fn removal_type_requiredness_enum_and_constraint_changes_break() {
    let previous = artifact(serde_json::json!({
        "type": "object",
        "properties": {
            "removed": {"type": "string"},
            "changed": {"type": "string"},
            "requiredness": {"type": "string"},
            "constrained": {"type": "string", "maxLength": 8},
            "choice": {"type": "string", "enum": ["a", "b"]}
        },
        "required": ["requiredness"]
    }));
    let current = artifact(serde_json::json!({
        "type": "object",
        "properties": {
            "renamed": {"type": "string"},
            "changed": {"type": "integer"},
            "requiredness": {"type": "string"},
            "constrained": {"type": "string", "maxLength": 9},
            "choice": {"type": "string", "enum": ["a", "b", "c"]}
        }
    }));
    let report = CatalogLock::from_artifact(&previous).check(&current);
    assert!(report.has_breaking());
    for (id, path) in [
        ("REC-COMPAT-014", "evidence_schema.properties.removed"),
        ("REC-COMPAT-012", "evidence_schema.properties.renamed"),
        ("REC-COMPAT-016", "evidence_schema.properties.changed.type"),
        ("REC-COMPAT-015", "evidence_schema.properties.requiredness"),
        (
            "REC-COMPAT-016",
            "evidence_schema.properties.constrained.maxLength",
        ),
        ("REC-COMPAT-016", "evidence_schema.properties.choice.enum"),
    ] {
        assert!(
            report
                .changes()
                .iter()
                .any(|change| change.id() == id && change.path() == path),
            "missing {id} at {path}"
        );
    }
}

#[test]
fn impact_schemas_follow_the_same_conservative_rules() {
    let previous_schema = serde_json::json!({
        "type": "object",
        "properties": {"artifact_count": {"type": "integer"}}
    });
    let current_schema = serde_json::json!({
        "type": "object",
        "properties": {"artifact_count": {"type": "integer"}},
        "required": ["artifact_count"]
    });
    let previous = with_impact(&previous_schema);
    let current = with_impact(&current_schema);
    let report = CatalogLock::from_artifact(&previous).check(&current);

    assert!(report.has_breaking());
    assert!(report.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-015"
            && change.path() == "surfaces.operation.impact_schema.properties.artifact_count"
    }));
}

#[test]
fn optional_addition_breaks_when_the_accepted_schema_rejects_unknown_fields() {
    let previous = artifact(serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    }));
    let current = artifact(serde_json::json!({
        "type": "object",
        "properties": {"added": {"type": "string"}},
        "additionalProperties": false
    }));
    let report = CatalogLock::from_artifact(&previous).check(&current);

    assert!(report.has_breaking());
    assert!(report.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-017" && change.path() == "evidence_schema.properties.added"
    }));
}

fn with_impact(schema: &serde_json::Value) -> CatalogArtifact {
    let baseline = artifact(serde_json::json!({"type": "object"}));
    let mut value = serde_json::to_value(baseline)
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"));
    value["diagnostics"][0]["surfaces"]["operation"] = serde_json::json!({"impact_schema": schema});
    parse(&value)
}
