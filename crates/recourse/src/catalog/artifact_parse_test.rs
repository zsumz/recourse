//! Bounded artifact parser round-trip and invariant rejection tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogArtifact, CatalogSpec, CodeNumber, ProblemSet};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Check the supplied job identifier.";
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}

fn artifact() -> CatalogArtifact {
    let get_job = ProblemSet::builder("getJob")
        .include::<JobNotFound>()
        .build();
    Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .problem_set(get_job)
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact()
}

fn encoded_value() -> serde_json::Value {
    serde_json::to_value(artifact())
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"))
}

fn parse_value(value: &serde_json::Value) -> Result<CatalogArtifact, super::ArtifactParseError> {
    let body = serde_json::to_vec(value)
        .unwrap_or_else(|error| panic!("mutated artifact must encode: {error}"));
    CatalogArtifact::from_slice(&body)
}

#[test]
fn generated_artifact_round_trips_through_bounded_parser() {
    let artifact = artifact();
    let mut body = Vec::new();
    assert!(artifact.write_pretty(&mut body).is_ok());

    assert_eq!(CatalogArtifact::from_slice(&body).ok(), Some(artifact));
}

#[test]
fn parser_rejects_version_identity_and_schema_drift() {
    let mut version = encoded_value();
    version["schema_version"] = serde_json::json!(2);
    assert!(parse_value(&version).is_err());

    let mut identity = encoded_value();
    identity["diagnostics"][0]["type"] = serde_json::json!("https://attacker.invalid/problem");
    assert!(parse_value(&identity).is_err());

    let mut schema = encoded_value();
    schema["diagnostics"][0]["evidence_schema"]["remote"] = serde_json::json!(true);
    assert!(parse_value(&schema).is_err());
}

#[test]
fn parser_rejects_unregistered_problem_set_codes() {
    let mut value = encoded_value();
    value["problem_sets"]["getJob"] = serde_json::json!(["DSP-1999"]);

    assert!(parse_value(&value).is_err());
}

#[test]
fn parser_rejects_statuses_without_their_mandatory_headers() {
    let mut value = encoded_value();
    value["diagnostics"][0]["surfaces"]["http"]["status"] = serde_json::json!(401);

    assert!(parse_value(&value).is_err());
}
