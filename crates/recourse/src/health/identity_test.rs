//! Health finding ID and RFC 3339 observation-time tests.

use time::{Duration, OffsetDateTime, UtcOffset};

use super::{HealthFindingId, MAX_HEALTH_FINDING_ID_BYTES, ObservationTime};

#[test]
fn canonical_finding_ids_round_trip() {
    let id = HealthFindingId::try_new("finding_01K2Y9H7M7V4WQ1Y8X3Z6A5B4C")
        .unwrap_or_else(|error| panic!("canonical ID must validate: {error}"));
    let json = serde_json::to_string(&id)
        .unwrap_or_else(|error| panic!("finding ID must serialize: {error}"));
    let decoded: HealthFindingId = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("finding ID must decode: {error}"));

    assert_eq!(decoded, id);
}

#[test]
fn malformed_finding_ids_are_rejected() {
    for value in ["", "finding_", "dia_123", "finding_has space"] {
        assert!(HealthFindingId::try_new(value).is_err(), "{value:?}");
    }
    let overlong = format!("finding_{}", "a".repeat(MAX_HEALTH_FINDING_ID_BYTES));
    assert!(HealthFindingId::try_new(overlong).is_err());
}

#[test]
fn observation_time_normalizes_offsets_and_round_trips() {
    let local = OffsetDateTime::from_unix_timestamp(1_786_372_260)
        .unwrap_or_else(|error| panic!("fixture timestamp must exist: {error}"))
        .to_offset(UtcOffset::from_hms(-5, 0, 0).unwrap_or(UtcOffset::UTC));
    let observed = ObservationTime::try_new(local)
        .unwrap_or_else(|error| panic!("fixture must format: {error}"));
    let decoded = ObservationTime::parse(observed.as_str())
        .unwrap_or_else(|error| panic!("canonical time must parse: {error}"));

    assert_eq!(observed.as_str(), "2026-08-10T14:31:00Z");
    assert_eq!(decoded, observed);
    assert_eq!(decoded.instant() - observed.instant(), Duration::ZERO);
}
