//! Exact, registration, dynamic-detail, and malicious-evidence Problem tests.

use std::borrow::Cow;

use http::{StatusCode, header::CONTENT_TYPE};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence, PublicText},
};

use super::{
    CorrelationId, Fixed, HttpProblemType, ProblemBuildError, ProblemEncodeError, ProblemOccurrence,
};

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "test";
    const PREFIX: &'static str = "TST";
    const TYPE_BASE: &'static str = "https://test.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct MissingEvidence {
    resource_id: String,
}

impl PublicEvidence for MissingEvidence {}

#[derive(Debug)]
enum MissingResource {}

impl DiagnosticType for MissingResource {
    type Catalog = TestCatalog;
    type Evidence = MissingEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Resource not found";
    const DETAIL: &'static str = "No resource exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the resource identifier."];
    const DOCS: &'static str = "The resource identifier is unknown.";
}

impl HttpProblemType for MissingResource {
    type Policy = Fixed<404>;
}

fn test_catalog() -> Option<Catalog<TestCatalog>> {
    Catalog::<TestCatalog>::builder()
        .problem::<MissingResource>()
        .build()
        .ok()
}

fn occurrence() -> Option<ProblemOccurrence> {
    ProblemOccurrence::new(
        CorrelationId::new("request-1").ok()?,
        "https://test.invalid/problem-occurrences/request-1",
    )
    .ok()
}

#[test]
fn fixed_problem_matches_the_canonical_wire_fixture() {
    let (Some(catalog), Some(occurrence)) = (test_catalog(), occurrence()) else {
        return;
    };
    let problem = catalog.try_problem::<MissingResource>(
        occurrence,
        MissingEvidence {
            resource_id: "r-1".to_owned(),
        },
    );
    let Some(problem) = problem.ok() else {
        return;
    };
    let encoded = problem.try_encode();
    let Some(encoded) = encoded.ok() else {
        return;
    };

    assert_eq!(encoded.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        encoded.headers().get(CONTENT_TYPE),
        Some(&http::HeaderValue::from_static("application/problem+json"))
    );
    let fixture = include_bytes!("../../tests/fixtures/wire/core-fixed-problem.json");
    assert_eq!(
        encoded.body(),
        fixture.strip_suffix(b"\n").unwrap_or(fixture)
    );
    assert!(
        crate::client::ReceivedProblem::from_slice(
            encoded.status(),
            encoded.headers(),
            encoded.body(),
            crate::wire::WireLimits::default(),
        )
        .is_ok()
    );
}

#[test]
fn dynamic_detail_requires_validated_public_text() {
    let (Some(catalog), Some(occurrence), Some(detail)) = (
        test_catalog(),
        occurrence(),
        PublicText::new("Resource r-1 was deleted.").ok(),
    ) else {
        return;
    };
    let problem = catalog.try_problem_with_detail::<MissingResource>(
        occurrence,
        MissingEvidence {
            resource_id: "r-1".to_owned(),
        },
        detail,
    );

    assert!(
        problem
            .ok()
            .and_then(|value| value.try_encode().ok())
            .is_some_and(|value| {
                String::from_utf8_lossy(value.body()).contains("Resource r-1 was deleted.")
            })
    );
}

#[derive(Debug)]
enum Unregistered {}

impl DiagnosticType for Unregistered {
    type Catalog = TestCatalog;
    type Evidence = MissingEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(2);
    const TITLE: &'static str = "Unregistered";
    const DETAIL: &'static str = "This declaration was not registered.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Not registered.";
}

impl HttpProblemType for Unregistered {
    type Policy = Fixed<400>;
}

#[test]
fn exact_unregistered_marker_is_rejected() {
    let (Some(catalog), Some(occurrence)) = (test_catalog(), occurrence()) else {
        return;
    };
    let result = catalog.try_problem::<Unregistered>(
        occurrence,
        MissingEvidence {
            resource_id: "r-1".to_owned(),
        },
    );

    assert!(matches!(
        result,
        Err(ProblemBuildError::DiagnosticNotRegistered { .. })
    ));
}

#[derive(Debug)]
struct ScalarEvidence;

impl Serialize for ScalarEvidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("not-an-object")
    }
}

impl JsonSchema for ScalarEvidence {
    fn schema_name() -> Cow<'static, str> {
        "ScalarEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "object" })
    }
}

impl PublicEvidence for ScalarEvidence {}

#[derive(Debug)]
enum DishonestEvidence {}

impl DiagnosticType for DishonestEvidence {
    type Catalog = TestCatalog;
    type Evidence = ScalarEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(3);
    const TITLE: &'static str = "Dishonest evidence";
    const DETAIL: &'static str = "The runtime serializer disagrees with its schema.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Runtime verification must reject this value.";
}

impl HttpProblemType for DishonestEvidence {
    type Policy = Fixed<500>;
}

#[test]
fn runtime_evidence_must_encode_as_an_object() {
    let Some(occurrence) = occurrence() else {
        return;
    };
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<DishonestEvidence>()
        .build();
    let result = catalog
        .ok()
        .and_then(|value| {
            value
                .try_problem::<DishonestEvidence>(occurrence, ScalarEvidence)
                .ok()
        })
        .and_then(|value| value.try_encode().err());

    assert!(matches!(
        result,
        Some(ProblemEncodeError::EvidenceNotObject)
    ));
}
