//! Strict health-finding construction and exact wire tests.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
};

use super::{
    HealthEncodeError, HealthFinding, HealthFindingId, HealthFindingType, HealthSeverity,
    ObservationTime,
};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "health-test";
    const PREFIX: &'static str = "HLT";
    const TYPE_BASE: &'static str = "https://health.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct QueueEvidence {
    consecutive_failures: u32,
}

impl PublicEvidence for QueueEvidence {}

enum QueueUnavailable {}

impl DiagnosticType for QueueUnavailable {
    type Catalog = TestCatalog;
    type Evidence = QueueEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1010);
    const TITLE: &'static str = "Job queue unavailable";
    const DETAIL: &'static str = "The worker cannot currently reach the job queue.";
    const SUGGESTIONS: &'static [&'static str] = &["Check queue connectivity."];
    const DOCS: &'static str = "Queue health failed.";
}

impl HealthFindingType for QueueUnavailable {}

fn fixture_finding() -> HealthFinding<QueueEvidence> {
    let catalog = Catalog::<TestCatalog>::builder()
        .health::<QueueUnavailable>()
        .build()
        .unwrap_or_else(|error| panic!("health catalog must build: {error}"));
    let id = HealthFindingId::try_new("finding_01KTEST")
        .unwrap_or_else(|error| panic!("fixture ID must validate: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture time must parse: {error}"));
    catalog
        .try_health::<QueueUnavailable>(
            id,
            HealthSeverity::Unhealthy,
            observed_at,
            QueueEvidence {
                consecutive_failures: 3,
            },
        )
        .unwrap_or_else(|error| panic!("registered finding must construct: {error}"))
}

#[test]
fn registered_health_finding_matches_the_canonical_wire_fixture() {
    let encoded = fixture_finding()
        .try_encode()
        .unwrap_or_else(|error| panic!("fixture must encode: {error}"));

    let fixture = include_bytes!("../../tests/fixtures/wire/core-health-finding.json");
    assert_eq!(encoded, fixture.strip_suffix(b"\n").unwrap_or(fixture));
    assert!(crate::client::decode_object(&encoded, crate::wire::WireLimits::default()).is_ok());
}

/// `serde_json::Value` compares members by name, so this pins the members and
/// their values, not the canonical byte order `try_encode` produces.
#[test]
fn the_value_encoder_carries_the_same_members_as_the_canonical_bytes() {
    let finding = fixture_finding();
    let encoded = finding
        .try_encode()
        .unwrap_or_else(|error| panic!("fixture must encode: {error}"));
    let value = finding
        .try_encode_value()
        .unwrap_or_else(|error| panic!("fixture must encode as a value: {error}"));

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&encoded).ok(),
        Some(value.clone())
    );
    assert_eq!(value["code"], "HLT-1010");
    assert_eq!(
        value["evidence"],
        serde_json::json!({ "consecutive_failures": 3 })
    );
}

#[test]
fn health_finding_requires_explicit_surface_registration() {
    let catalog = Catalog::<TestCatalog>::builder()
        .build()
        .unwrap_or_else(|error| panic!("empty catalog must build: {error}"));
    let id = HealthFindingId::try_new("finding_missing")
        .unwrap_or_else(|error| panic!("fixture ID must validate: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture time must parse: {error}"));
    let result = catalog.try_health::<QueueUnavailable>(
        id,
        HealthSeverity::Degraded,
        observed_at,
        QueueEvidence {
            consecutive_failures: 1,
        },
    );

    assert!(result.is_err());
}

#[derive(Debug)]
struct WrongHealthEvidence;

impl Serialize for WrongHealthEvidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("consecutive_failures", "many")?;
        map.end()
    }
}

impl JsonSchema for WrongHealthEvidence {
    fn schema_name() -> Cow<'static, str> {
        "WrongHealthEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {"consecutive_failures": {"type": "integer"}},
            "required": ["consecutive_failures"],
            "additionalProperties": false
        })
    }
}

impl PublicEvidence for WrongHealthEvidence {}

enum DishonestFinding {}

impl DiagnosticType for DishonestFinding {
    type Catalog = TestCatalog;
    type Evidence = WrongHealthEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1011);
    const TITLE: &'static str = "Dishonest finding";
    const DETAIL: &'static str = "Runtime evidence disagrees with its schema.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Runtime finding conformance test.";
}

impl HealthFindingType for DishonestFinding {}

#[test]
fn finding_evidence_must_match_its_precompiled_schema() {
    let catalog = Catalog::<TestCatalog>::builder()
        .health::<DishonestFinding>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"));
    let id = HealthFindingId::try_new("finding_schema")
        .unwrap_or_else(|error| panic!("fixture ID must validate: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture time must parse: {error}"));
    let error = catalog
        .try_health::<DishonestFinding>(
            id,
            HealthSeverity::Degraded,
            observed_at,
            WrongHealthEvidence,
        )
        .ok()
        .and_then(|finding| finding.try_encode().err());

    assert!(matches!(
        error,
        Some(HealthEncodeError::EvidenceSchemaMismatch { path, .. })
            if path == "$/consecutive_failures"
    ));
}
