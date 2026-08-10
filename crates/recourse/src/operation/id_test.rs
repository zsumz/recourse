//! Durable diagnostic ID validation and wire round-trip tests.

use super::{MAX_OPERATION_DIAGNOSTIC_ID_BYTES, OperationDiagnosticId};

#[test]
fn canonical_ids_round_trip_through_json() {
    let id = OperationDiagnosticId::try_new("dia_01K2Y9H7M7V4WQ1Y8X3Z6A5B4C")
        .unwrap_or_else(|error| panic!("canonical ID must validate: {error}"));
    let json = serde_json::to_string(&id)
        .unwrap_or_else(|error| panic!("canonical ID must serialize: {error}"));
    let decoded: OperationDiagnosticId = serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("canonical ID must decode: {error}"));

    assert_eq!(decoded, id);
    assert_eq!(id.as_str(), "dia_01K2Y9H7M7V4WQ1Y8X3Z6A5B4C");
}

#[test]
fn malformed_ids_are_rejected() {
    for value in ["", "dia_", "job_123", "dia_has space", "dia_☃"] {
        assert!(OperationDiagnosticId::try_new(value).is_err(), "{value:?}");
    }
    let overlong = format!("dia_{}", "a".repeat(MAX_OPERATION_DIAGNOSTIC_ID_BYTES));
    assert!(OperationDiagnosticId::try_new(overlong).is_err());
}
