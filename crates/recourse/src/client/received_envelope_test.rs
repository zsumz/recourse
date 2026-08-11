//! Tolerant operation and health envelope preservation tests.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    health::{HealthFindingType, HealthSeverity},
    operation::OperationDiagnosticType,
};

use super::{
    Classification, DecodeLimits, ProtocolIssue, ReceivedHealthFinding, ReceivedOperationDiagnostic,
};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "received-envelope";
    const PREFIX: &'static str = "REN";
    const TYPE_BASE: &'static str = "https://received.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct Impact {
    unchanged: bool,
}

impl PublicEvidence for Impact {}

enum SharedDiagnostic {}

impl DiagnosticType for SharedDiagnostic {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(9);
    const TITLE: &'static str = "Shared diagnostic";
    const DETAIL: &'static str = "Shared across non-HTTP surfaces.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Shared fixture.";
}

impl OperationDiagnosticType for SharedDiagnostic {
    type Impact = Impact;
}

impl HealthFindingType for SharedDiagnostic {}

#[test]
fn operation_preserves_unknown_and_wrong_typed_members() {
    let body = br#"{"id":"dia_01KTEST","type":"https://received.invalid/problems/REN-9","code":"REN-9","evidence":{},"impact":{"unchanged":true,"future":7},"suggestions":["one",2],"vendor":true}"#;
    let received = ReceivedOperationDiagnostic::from_slice(body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("operation fixture must decode: {error}"));
    let catalog = Catalog::<TestCatalog>::builder()
        .operation::<SharedDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("operation catalog must build: {error}"));

    assert_eq!(
        received.id().map(ToString::to_string).as_deref(),
        Some("dia_01KTEST")
    );
    assert_eq!(
        received
            .impact()
            .and_then(|value| value.get("future"))
            .and_then(serde_json::Value::as_i64),
        Some(7)
    );
    assert_eq!(received.suggestions(), ["one"]);
    assert_eq!(
        received.protocol_issues(),
        [ProtocolIssue::InvalidMemberType {
            member: "suggestions",
            expected: "array of strings"
        }]
    );
    assert_eq!(received.raw()["vendor"], true);
    assert!(matches!(
        catalog.classify_operation(&received),
        Classification::Known(_)
    ));
}

#[test]
fn health_parses_semantics_and_keeps_future_data() {
    let body = br#"{"id":"finding_01KTEST","type":"https://received.invalid/problems/REN-9","code":"REN-9","severity":"unhealthy","observed_at":"2026-08-10T09:31:00-05:00","evidence":{},"future":{"kept":true}}"#;
    let received = ReceivedHealthFinding::from_slice(body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("health fixture must decode: {error}"));
    let catalog = Catalog::<TestCatalog>::builder()
        .health::<SharedDiagnostic>()
        .build()
        .unwrap_or_else(|error| panic!("health catalog must build: {error}"));

    assert_eq!(received.severity(), Some(HealthSeverity::Unhealthy));
    assert_eq!(
        received.observed_at().map(ToString::to_string).as_deref(),
        Some("2026-08-10T14:31:00Z")
    );
    assert_eq!(received.raw()["future"]["kept"], true);
    assert!(matches!(
        catalog.classify_health(&received),
        Classification::Known(_)
    ));
}

#[test]
fn malformed_surface_members_are_nonfatal_issues() {
    let operation = ReceivedOperationDiagnostic::from_slice(
        br#"{"id":"bad","code":"REN-9","impact":[]}"#,
        DecodeLimits::default(),
    )
    .unwrap_or_else(|error| panic!("operation fallback must decode: {error}"));
    let health = ReceivedHealthFinding::from_slice(
        br#"{"id":"bad","severity":"healthy","observed_at":"yesterday","code":"REN-9"}"#,
        DecodeLimits::default(),
    )
    .unwrap_or_else(|error| panic!("health fallback must decode: {error}"));

    assert_eq!(operation.impact(), None);
    assert_eq!(
        operation.protocol_issues(),
        [
            ProtocolIssue::MalformedOperationDiagnosticId,
            ProtocolIssue::InvalidMemberType {
                member: "impact",
                expected: "object"
            }
        ]
    );
    assert_eq!(
        health.protocol_issues(),
        [
            ProtocolIssue::MalformedHealthFindingId,
            ProtocolIssue::InvalidHealthSeverity,
            ProtocolIssue::InvalidObservationTime
        ]
    );
}
