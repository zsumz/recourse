//! Runtime schema and wire-profile conformance tests.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
    wire::{WireLimit, WireLimits},
};

use super::{CorrelationId, Fixed, HttpProblemType, ProblemEncodeError, ProblemOccurrence};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "contract-test";
    const PREFIX: &'static str = "CON";
    const TYPE_BASE: &'static str = "https://contract.invalid/problems/";
}

fn occurrence() -> ProblemOccurrence {
    ProblemOccurrence::new(
        CorrelationId::new("request-1")
            .unwrap_or_else(|error| panic!("fixture correlation ID must validate: {error}")),
        "/problem-occurrences/request-1",
    )
    .unwrap_or_else(|error| panic!("fixture occurrence must validate: {error}"))
}

#[derive(Debug)]
struct WrongObjectEvidence;

impl Serialize for WrongObjectEvidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("answer", "not-an-integer")?;
        map.end()
    }
}

impl JsonSchema for WrongObjectEvidence {
    fn schema_name() -> Cow<'static, str> {
        "WrongObjectEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {"answer": {"type": "integer"}},
            "required": ["answer"],
            "additionalProperties": false
        })
    }
}

impl PublicEvidence for WrongObjectEvidence {}

enum SchemaMismatch {}

impl DiagnosticType for SchemaMismatch {
    type Catalog = TestCatalog;
    type Evidence = WrongObjectEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Schema mismatch";
    const DETAIL: &'static str = "The serializer disagrees with the accepted schema.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Runtime schema conformance test.";
}

impl HttpProblemType for SchemaMismatch {
    type Policy = Fixed<500>;
}

#[test]
fn object_evidence_must_match_its_precompiled_schema() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<SchemaMismatch>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"));
    let error = catalog
        .try_problem::<SchemaMismatch>(occurrence(), WrongObjectEvidence)
        .ok()
        .and_then(|problem| problem.try_encode().err());

    assert!(matches!(
        error,
        Some(ProblemEncodeError::EvidenceSchemaMismatch { path, .. }) if path == "$/answer"
    ));
}

#[derive(Debug, Serialize, JsonSchema)]
struct LargeEvidence {
    payload: String,
}

impl PublicEvidence for LargeEvidence {}

enum WireLimitDiagnostic {}

impl DiagnosticType for WireLimitDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = LargeEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(2);
    const TITLE: &'static str = "Wire limit";
    const DETAIL: &'static str = "Oversized evidence must fail closed.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Shared wire-profile test.";
}

impl HttpProblemType for WireLimitDiagnostic {
    type Policy = Fixed<500>;
}

#[test]
fn emitters_enforce_the_same_string_limit_as_default_clients() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<WireLimitDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"));
    let error = catalog
        .try_problem::<WireLimitDiagnostic>(
            occurrence(),
            LargeEvidence {
                payload: "x".repeat(WireLimits::DEFAULT_MAX_STRING_BYTES + 1),
            },
        )
        .ok()
        .and_then(|problem| problem.try_encode().err());

    assert!(matches!(
        error,
        Some(ProblemEncodeError::WireLimit(error)) if error.limit() == WireLimit::StringBytes
    ));
}

#[derive(Debug, Serialize, JsonSchema)]
struct LargeBodyEvidence {
    payloads: Vec<String>,
}

impl PublicEvidence for LargeBodyEvidence {}

enum BodyLimitDiagnostic {}

impl DiagnosticType for BodyLimitDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = LargeBodyEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(3);
    const TITLE: &'static str = "Body limit";
    const DETAIL: &'static str = "The final writer must remain capped.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Capped writer test.";
}

impl HttpProblemType for BodyLimitDiagnostic {
    type Policy = Fixed<500>;
}

#[test]
fn canonical_json_writer_stops_at_the_shared_body_limit() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<BodyLimitDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"));
    let error = catalog
        .try_problem::<BodyLimitDiagnostic>(
            occurrence(),
            LargeBodyEvidence {
                payloads: vec!["x".repeat(700); 100],
            },
        )
        .ok()
        .and_then(|problem| problem.try_encode().err());

    assert!(matches!(
        error,
        Some(ProblemEncodeError::WireLimit(error)) if error.limit() == WireLimit::BodyBytes
    ));
}
