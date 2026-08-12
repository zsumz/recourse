//! Bounded artifact parser round-trip and invariant rejection tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
    wire::WireLimits,
};

use super::{Catalog, CatalogArtifact, CatalogLock, CatalogSpec, CodeNumber, ProblemSet};

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
fn parser_rejects_duplicate_members_before_deserialization() {
    let mut body = Vec::new();
    artifact()
        .write_pretty(&mut body)
        .unwrap_or_else(|error| panic!("fixture artifact must encode: {error}"));
    let encoded = String::from_utf8(body)
        .unwrap_or_else(|error| panic!("JSON fixture must be UTF-8: {error}"));
    let duplicated = encoded.replacen(
        "  \"schema_version\": 1,",
        "  \"schema_version\": 1,\n  \"schema_version\": 1,",
        1,
    );

    let error = CatalogArtifact::from_slice(duplicated.as_bytes())
        .err()
        .unwrap_or_else(|| panic!("duplicate member must be rejected"));
    assert!(error.to_string().contains("duplicate JSON member"));
}

#[test]
fn lock_parser_rejects_duplicate_members_before_deserialization() {
    let mut body = Vec::new();
    CatalogLock::from_artifact(&artifact())
        .write_pretty(&mut body)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    let encoded = String::from_utf8(body)
        .unwrap_or_else(|error| panic!("JSON fixture must be UTF-8: {error}"));
    let duplicated = encoded.replacen(
        "  \"schema_version\": 1,",
        "  \"schema_version\": 1,\n  \"schema_version\": 1,",
        1,
    );

    let error = CatalogLock::from_slice(duplicated.as_bytes())
        .err()
        .unwrap_or_else(|| panic!("duplicate member must be rejected"));
    assert!(error.to_string().contains("duplicate JSON member"));
}

#[test]
fn parser_rejects_version_identity_and_schema_drift() {
    let mut version = encoded_value();
    version["schema_version"] = serde_json::json!(2);
    assert!(parse_value(&version).is_err());

    let mut identity = encoded_value();
    identity["diagnostics"][0]["type"] = serde_json::json!("https://attacker.invalid/problem");
    assert!(parse_value(&identity).is_err());

    for type_base in [
        "https://dispatch.invalid/problems?next=/",
        "https://dispatch.invalid/problems#/",
    ] {
        let mut invalid_base = encoded_value();
        invalid_base["catalog"]["type_base"] = serde_json::json!(type_base);
        assert!(parse_value(&invalid_base).is_err());
    }

    let mut schema = encoded_value();
    schema["diagnostics"][0]["evidence_schema"]["remote"] = serde_json::json!(true);
    assert!(parse_value(&schema).is_err());
}

#[test]
fn parser_rejects_an_empty_artifact_with_an_exhausted_type_namespace() {
    let mut value = encoded_value();
    value["catalog"]["type_base"] = serde_json::json!(capacity_type_base("DSP"));
    value["diagnostics"] = serde_json::json!([]);
    value["problem_sets"] = serde_json::json!({});

    let error = parse_value(&value).err();
    assert!(error.is_some_and(|error| error.to_string().contains("largest code")));
}

fn capacity_type_base(prefix: &str) -> String {
    let one_digit_code_bytes = prefix.len() + 2;
    let base_bytes = WireLimits::DEFAULT_MAX_STRING_BYTES - one_digit_code_bytes;
    format!("https://{}/", "a".repeat(base_bytes - 8))
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

#[test]
fn parser_reapplies_static_metadata_bounds() {
    let mut control = encoded_value();
    control["diagnostics"][0]["detail"] = serde_json::json!("unsafe\u{7}detail");
    assert!(parse_value(&control).is_err());

    let mut suggestions = encoded_value();
    suggestions["diagnostics"][0]["suggestions"] =
        serde_json::json!(vec!["help"; super::MAX_SUGGESTIONS + 1]);
    assert!(parse_value(&suggestions).is_err());
}
