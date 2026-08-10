//! Namespace, reservation, and permanent-history compatibility fixtures.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogArtifact, CatalogLock, CatalogSpec, CodeNumber, Reservation};

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

#[test]
fn a_valid_but_different_namespace_is_forbidden() {
    let mut value = serde_json::to_value(artifact())
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"));
    value["catalog"]["name"] = serde_json::json!("other-dispatch");
    value["catalog"]["prefix"] = serde_json::json!("OTH");
    value["catalog"]["type_base"] = serde_json::json!("https://other.invalid/problems/");
    value["diagnostics"][0]["code"] = serde_json::json!("OTH-1001");
    value["diagnostics"][0]["type"] = serde_json::json!("https://other.invalid/problems/OTH-1001");
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("changed namespace must encode: {error}"));
    let current = CatalogArtifact::from_slice(&body)
        .unwrap_or_else(|error| panic!("changed namespace must remain valid: {error}"));
    let report = CatalogLock::from_artifact(&artifact()).check(&current);

    assert!(report.has_forbidden());
    assert!(
        report
            .changes()
            .iter()
            .all(|change| change.id() == "REC-COMPAT-001")
    );
}

#[test]
fn accepting_a_matching_reservation_activates_it() {
    let mut lock = CatalogLock::from_artifact(&artifact());
    let reserved = lock
        .reserve(Reservation::Exact(CodeNumber::new(1002)))
        .unwrap_or_else(|error| panic!("unused identity must reserve: {error}"));
    assert_eq!(reserved.code().to_string(), "DSP-1002");

    let mut value = serde_json::to_value(artifact())
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"));
    let mut added = value["diagnostics"][0].clone();
    added["number"] = serde_json::json!(1002);
    added["code"] = serde_json::json!("DSP-1002");
    added["type"] = serde_json::json!("https://dispatch.invalid/problems/DSP-1002");
    value["diagnostics"]
        .as_array_mut()
        .unwrap_or_else(|| panic!("fixture diagnostics must be an array"))
        .push(added);
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("activated artifact must encode: {error}"));
    let current = CatalogArtifact::from_slice(&body)
        .unwrap_or_else(|error| panic!("activated artifact must parse: {error}"));
    let report = lock.check(&current);

    assert!(report.is_compatible());
    assert!(
        report
            .changes()
            .iter()
            .any(|change| change.id() == "REC-COMPAT-005")
    );
}
