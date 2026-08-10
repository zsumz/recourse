//! Conservative compatibility fixtures and explicit acceptance lifecycle tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{
    AcceptanceError, AcceptanceMode, Catalog, CatalogArtifact, CatalogLock, CatalogSpec, Code,
    CodeNumber, CompatibilitySeverity,
};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum MalformedRequest {}

impl DiagnosticType for MalformedRequest {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1001);
    const TITLE: &'static str = "Malformed request";
    const DETAIL: &'static str = "The request could not be parsed.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Send valid JSON.";
}

impl HttpProblemType for MalformedRequest {
    type Policy = Fixed<400>;
}

fn artifact() -> CatalogArtifact {
    Catalog::<DispatchCatalog>::builder()
        .problem::<MalformedRequest>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact()
}

fn mutate(mutator: impl FnOnce(&mut serde_json::Value)) -> CatalogArtifact {
    let mut value = serde_json::to_value(artifact())
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"));
    mutator(&mut value);
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("mutated artifact must encode: {error}"));
    CatalogArtifact::from_slice(&body)
        .unwrap_or_else(|error| panic!("mutated artifact must remain valid: {error}"))
}

fn add_property(value: &mut serde_json::Value, required: bool) {
    let schema = &mut value["diagnostics"][0]["evidence_schema"];
    schema["properties"] = serde_json::json!({"trace_id": {"type": "string"}});
    if required {
        schema["required"] = serde_json::json!(["trace_id"]);
    }
}

fn code() -> Code {
    "DSP-1001"
        .parse()
        .unwrap_or_else(|error| panic!("fixture code must parse: {error}"))
}

#[test]
fn optional_field_is_compatible_and_accepts_without_acknowledgement() {
    let current = mutate(|value| add_property(value, false));
    let mut lock = CatalogLock::from_artifact(&artifact());
    let report = lock.check(&current);

    assert!(report.is_compatible());
    super::compatibility_profile_test::assert_report_fixture(
        &report,
        include_str!("../../../../conformance/compatibility/compatible-report.json"),
    );
    assert!(
        report
            .changes()
            .iter()
            .any(|change| change.id() == "REC-COMPAT-012")
    );
    assert!(
        lock.accept(&current, AcceptanceMode::CompatibleOnly)
            .is_ok()
    );
    assert!(lock.check(&current).changes().is_empty());
}

#[test]
fn required_field_needs_explicit_breaking_acknowledgement() {
    let current = mutate(|value| add_property(value, true));
    let mut lock = CatalogLock::from_artifact(&artifact());
    let report = lock.check(&current);

    assert!(report.has_breaking());
    super::compatibility_profile_test::assert_report_fixture(
        &report,
        include_str!("../../../../conformance/compatibility/breaking-report.json"),
    );
    assert!(report.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-013" && change.severity() == CompatibilitySeverity::Breaking
    }));
    assert!(matches!(
        lock.accept(&current, AcceptanceMode::CompatibleOnly),
        Err(AcceptanceError::BreakingRequiresAcknowledgement(_))
    ));
    assert!(
        lock.accept(&current, AcceptanceMode::AcknowledgeBreaking)
            .is_ok()
    );
}

#[test]
fn title_and_http_status_changes_are_breaking() {
    let current = mutate(|value| {
        value["diagnostics"][0]["title"] = serde_json::json!("Invalid request");
        value["diagnostics"][0]["surfaces"]["http"]["status"] = serde_json::json!(409);
    });
    let lock = CatalogLock::from_artifact(&artifact());
    let report = lock.check(&current);
    let ids = report
        .changes()
        .iter()
        .map(super::CompatibilityChange::id)
        .collect::<Vec<_>>();

    assert!(ids.contains(&"REC-COMPAT-006"));
    assert!(ids.contains(&"REC-COMPAT-010"));
    assert!(report.has_breaking());
}

#[test]
fn deletion_requires_retirement_and_retired_reuse_is_forbidden() {
    let absent = mutate(|value| value["diagnostics"] = serde_json::json!([]));
    let mut lock = CatalogLock::from_artifact(&artifact());
    let deletion = lock.check(&absent);
    assert!(deletion.has_forbidden());
    assert!(
        deletion
            .changes()
            .iter()
            .any(|change| change.id() == "REC-COMPAT-003")
    );

    assert!(lock.retire(&code(), "Superseded endpoint", None).is_ok());
    assert!(lock.check(&absent).changes().is_empty());
    let reused = lock.check(&artifact());
    assert!(reused.has_forbidden());
    assert!(
        reused
            .changes()
            .iter()
            .any(|change| change.id() == "REC-COMPAT-002")
    );
}

#[test]
fn diagnostic_and_surface_additions_are_compatible() {
    let current = mutate(|value| {
        let diagnostic = &mut value["diagnostics"][0];
        diagnostic["surfaces"]["operation"] = serde_json::json!({
            "impact_schema": {"type": "object"}
        });
        diagnostic["surfaces"]["health"] = serde_json::json!({});
        let mut added = diagnostic.clone();
        added["number"] = serde_json::json!(1002);
        added["code"] = serde_json::json!("DSP-1002");
        added["type"] = serde_json::json!("https://dispatch.invalid/problems/DSP-1002");
        value["diagnostics"]
            .as_array_mut()
            .unwrap_or_else(|| panic!("fixture diagnostics must be an array"))
            .push(added);
    });
    let report = CatalogLock::from_artifact(&artifact()).check(&current);
    let ids = report
        .changes()
        .iter()
        .map(super::CompatibilityChange::id)
        .collect::<Vec<_>>();

    assert!(report.is_compatible());
    assert!(ids.contains(&"REC-COMPAT-004"));
    assert_eq!(ids.iter().filter(|id| **id == "REC-COMPAT-008").count(), 2);
}

#[test]
fn every_guidance_field_may_improve_compatibly() {
    for field in ["detail", "suggestions", "documentation_markdown"] {
        let current = mutate(|value| match field {
            "detail" => value["diagnostics"][0][field] = serde_json::json!("Improved detail."),
            "suggestions" => {
                value["diagnostics"][0][field] = serde_json::json!(["Improved suggestion."]);
            }
            "documentation_markdown" => {
                value["diagnostics"][0][field] = serde_json::json!("Improved documentation.");
            }
            _ => panic!("unknown guidance field"),
        });
        let report = CatalogLock::from_artifact(&artifact()).check(&current);

        assert!(report.is_compatible(), "{field}");
        assert!(
            report
                .changes()
                .iter()
                .any(|change| change.id() == "REC-COMPAT-007"),
            "{field}"
        );
    }
}

#[test]
fn removed_surfaces_and_changed_http_headers_are_breaking() {
    let previous = mutate(|value| {
        value["diagnostics"][0]["surfaces"]["operation"] = serde_json::json!({
            "impact_schema": {"type": "object"}
        });
        value["diagnostics"][0]["surfaces"]["health"] = serde_json::json!({});
    });
    let current = mutate(|value| {
        value["diagnostics"][0]["surfaces"]["http"]["required_headers"] =
            serde_json::json!(["retry-after"]);
    });
    let report = CatalogLock::from_artifact(&previous).check(&current);
    let ids = report
        .changes()
        .iter()
        .map(super::CompatibilityChange::id)
        .collect::<Vec<_>>();

    assert!(report.has_breaking());
    assert_eq!(ids.iter().filter(|id| **id == "REC-COMPAT-009").count(), 2);
    assert!(ids.contains(&"REC-COMPAT-011"));
}
