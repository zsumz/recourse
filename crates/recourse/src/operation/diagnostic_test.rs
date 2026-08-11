//! Strict durable-diagnostic construction and exact wire tests.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
};

use super::{OperationDiagnostic, OperationDiagnosticId, OperationDiagnosticType};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "operation-test";
    const PREFIX: &'static str = "OPT";
    const TYPE_BASE: &'static str = "https://operation.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct FailureEvidence {
    attempt: u32,
}

impl PublicEvidence for FailureEvidence {}

#[derive(Debug, Serialize, JsonSchema)]
struct FailureImpact {
    destination_changed: bool,
    created_artifacts: u32,
}

impl PublicEvidence for FailureImpact {}

enum DispatchFailed {}

impl DiagnosticType for DispatchFailed {
    type Catalog = TestCatalog;
    type Evidence = FailureEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1009);
    const TITLE: &'static str = "Job dispatch failed";
    const DETAIL: &'static str = "The accepted job could not be dispatched.";
    const SUGGESTIONS: &'static [&'static str] = &["Inspect the failed attempt."];
    const DOCS: &'static str = "Dispatch failed after request acceptance.";
}

impl OperationDiagnosticType for DispatchFailed {
    type Impact = FailureImpact;
}

fn fixture_diagnostic() -> OperationDiagnostic<FailureEvidence, FailureImpact> {
    let catalog = Catalog::<TestCatalog>::builder()
        .operation::<DispatchFailed>()
        .build()
        .unwrap_or_else(|error| panic!("operation catalog must build: {error}"));
    let id = OperationDiagnosticId::try_new("dia_01KTEST")
        .unwrap_or_else(|error| panic!("fixture ID must validate: {error}"));
    catalog
        .try_operation::<DispatchFailed>(
            id,
            FailureEvidence { attempt: 3 },
            FailureImpact {
                destination_changed: false,
                created_artifacts: 2,
            },
        )
        .unwrap_or_else(|error| panic!("registered operation must construct: {error}"))
}

#[test]
fn registered_operation_matches_the_canonical_wire_fixture() {
    let encoded = fixture_diagnostic()
        .try_encode()
        .unwrap_or_else(|error| panic!("fixture must encode: {error}"));

    let fixture = include_bytes!("../../tests/fixtures/wire/core-operation.json");
    assert_eq!(encoded, fixture.strip_suffix(b"\n").unwrap_or(fixture));
}

/// `serde_json::Value` compares members by name, so this pins the members and
/// their values, not the canonical byte order `try_encode` produces.
#[test]
fn the_value_encoder_carries_the_same_members_as_the_canonical_bytes() {
    let diagnostic = fixture_diagnostic();
    let encoded = diagnostic
        .try_encode()
        .unwrap_or_else(|error| panic!("fixture must encode: {error}"));
    let value = diagnostic
        .try_encode_value()
        .unwrap_or_else(|error| panic!("fixture must encode as a value: {error}"));

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&encoded).ok(),
        Some(value.clone())
    );
    assert_eq!(value["code"], "OPT-1009");
    assert_eq!(value["evidence"], serde_json::json!({ "attempt": 3 }));
}

#[test]
fn operation_requires_explicit_surface_registration() {
    let catalog = Catalog::<TestCatalog>::builder()
        .build()
        .unwrap_or_else(|error| panic!("empty catalog must build: {error}"));
    let id = OperationDiagnosticId::try_new("dia_missing")
        .unwrap_or_else(|error| panic!("fixture ID must validate: {error}"));
    let result = catalog.try_operation::<DispatchFailed>(
        id,
        FailureEvidence { attempt: 1 },
        FailureImpact {
            destination_changed: false,
            created_artifacts: 0,
        },
    );

    assert!(result.is_err());
}
