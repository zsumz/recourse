//! Multi-surface catalog merging and impact-schema validation tests.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;

use crate::{
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    health::HealthFindingType,
    http::{Fixed, HttpProblemType},
    operation::OperationDiagnosticType,
};

use super::{
    Catalog, CatalogIssue, CatalogSpec, CodeNumber,
    schema::number::{is_public, values_equal},
};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "surface-test";
    const PREFIX: &'static str = "SUR";
    const TYPE_BASE: &'static str = "https://surface.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct QueueImpact {
    accepted_work_unchanged: bool,
}

impl PublicEvidence for QueueImpact {}

enum QueueUnavailable {}

impl DiagnosticType for QueueUnavailable {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(10);
    const TITLE: &'static str = "Queue unavailable";
    const DETAIL: &'static str = "The queue cannot accept work.";
    const SUGGESTIONS: &'static [&'static str] = &["Check queue connectivity."];
    const DOCS: &'static str = "Queue connectivity failed.";
}

impl HttpProblemType for QueueUnavailable {
    type Policy = Fixed<503>;
}

impl OperationDiagnosticType for QueueUnavailable {
    type Impact = QueueImpact;
}

impl HealthFindingType for QueueUnavailable {}

#[test]
fn one_identity_merges_three_explicit_surfaces() {
    let catalog = Catalog::<TestCatalog>::builder()
        .health::<QueueUnavailable>()
        .problem::<QueueUnavailable>()
        .operation::<QueueUnavailable>()
        .build()
        .unwrap_or_else(|error| panic!("multi-surface catalog must build: {error}"));
    let artifact = catalog.artifact();
    let Some(diagnostic) = artifact.diagnostics().first() else {
        panic!("registered diagnostic must exist");
    };

    assert_eq!(artifact.diagnostics().len(), 1);
    assert_eq!(diagnostic.http_status(), Some(503));
    assert_eq!(diagnostic.http_policy(), Some("fixed"));
    assert_eq!(diagnostic.required_headers(), Some(&[] as &[String]));
    assert!(diagnostic.supports_health());
    assert_eq!(
        diagnostic
            .impact_schema()
            .and_then(|schema| schema.pointer("/properties/accepted_work_unchanged/type"))
            .and_then(serde_json::Value::as_str),
        Some("boolean")
    );
}

#[derive(Debug, Serialize, JsonSchema)]
struct ScalarImpact(String);

impl PublicEvidence for ScalarImpact {}

enum InvalidImpact {}

impl DiagnosticType for InvalidImpact {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(11);
    const TITLE: &'static str = "Invalid impact";
    const DETAIL: &'static str = "The impact declaration is invalid.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Invalid test declaration.";
}

impl OperationDiagnosticType for InvalidImpact {
    type Impact = ScalarImpact;
}

#[test]
fn operation_impact_must_use_the_public_object_profile() {
    let error = Catalog::<TestCatalog>::builder()
        .operation::<InvalidImpact>()
        .build()
        .err();

    assert!(error.is_some_and(|error| {
        error.issues().iter().any(|issue| {
            matches!(
                issue,
                CatalogIssue::UnsupportedImpactSchema { number, .. }
                    if *number == CodeNumber::new(11)
            )
        })
    }));
}

#[derive(Debug, Serialize)]
struct UnemittableImpact;

impl PublicEvidence for UnemittableImpact {}

impl JsonSchema for UnemittableImpact {
    fn schema_name() -> Cow<'static, str> {
        "UnemittableImpact".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        serde_json::from_str(
            r#"{"type":"object","properties":{"value":{"type":"number","const":1e400}}}"#,
        )
        .unwrap_or_else(|error| panic!("exact impact schema must parse: {error}"))
    }
}

enum ImpossibleImpact {}

impl DiagnosticType for ImpossibleImpact {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(12);
    const TITLE: &'static str = "Impossible impact";
    const DETAIL: &'static str = "The impact cannot be emitted.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Impossible numeric impact fixture.";
}

impl OperationDiagnosticType for ImpossibleImpact {
    type Impact = UnemittableImpact;
}

#[test]
fn operation_impact_cannot_require_an_unemittable_number() {
    let error = Catalog::<TestCatalog>::builder()
        .operation::<ImpossibleImpact>()
        .build()
        .err();
    assert!(
        error.is_some_and(|error| error.issues().iter().any(|issue| matches!(
            issue,
            CatalogIssue::UnsupportedImpactSchema { number, .. } if *number == CodeNumber::new(12)
        )))
    );
}

fn exact_number(encoded: &str) -> serde_json::Number {
    serde_json::from_str(encoded).unwrap_or_else(|error| panic!("exact number must parse: {error}"))
}

#[test]
fn public_numeric_domain_matches_primitive_emitters() {
    for encoded in [
        "-9223372036854775808",
        "-9223372036854775808.0",
        "18446744073709551615",
        "9007199254740993.0",
        "9007199254740993000e-3",
        "18446744073709551615e0",
        "1844674407370955161500e-2",
        "-9007199254740993.000",
        "0.1",
        "3.4028235e38",
        "1.7976931348623157e308",
    ] {
        assert!(
            is_public(&exact_number(encoded), "$").unwrap_or(false),
            "{encoded}"
        );
    }
    for encoded in [
        "18446744073709551616",
        "18446744073709551616.0",
        "1844674407370955161600e-2",
        "-9223372036854775809.0",
        "0.100000000000000000001",
        "1e400",
    ] {
        assert!(
            !is_public(&exact_number(encoded), "$").unwrap_or(true),
            "{encoded}"
        );
    }
}

#[test]
fn equivalent_decimal_spellings_are_exactly_equal() {
    let integer = serde_json::Value::Number(exact_number("1"));
    let decimal = serde_json::Value::Number(exact_number("1.00"));
    assert!(values_equal(&integer, &decimal));
}
