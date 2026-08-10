//! Exact aggregate health endpoint over one typed worker finding.

use std::time::Duration;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use dispatch_service::{JobAdmission, QueueObservation};
use recourse::{
    client::{DecodeLimits, ReceivedHealthFinding},
    health::{HealthFindingId, HealthSeverity, ObservationTime},
};
use tower::ServiceExt;

fn admission() -> JobAdmission {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("fixture finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture observation time must be valid: {error}"));
    JobAdmission::new(
        QueueObservation::new(finding_id, HealthSeverity::Degraded, observed_at, 3),
        16,
        Duration::from_secs(30),
    )
}

#[tokio::test]
async fn health_endpoint_returns_exact_typed_findings() {
    let app = dispatch_api_axum::router_with_admission(&admission())
        .unwrap_or_else(|error| panic!("test router must build: {error}"));
    let request = Request::get("/health")
        .header("x-request-id", "health-test-request")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("health request must build: {error}"));
    let response = app
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-request-id"], "health-test-request");
    let body = to_bytes(response.into_body(), 8192)
        .await
        .unwrap_or_else(|error| panic!("health body must be readable: {error}"));
    let fixture = include_bytes!("../../../conformance/wire/dispatch-health-resource.json");
    assert_eq!(
        body.as_ref(),
        fixture.strip_suffix(b"\n").unwrap_or(fixture)
    );

    let resource: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("health resource must decode: {error}"));
    let finding = serde_json::to_vec(&resource["findings"][0])
        .unwrap_or_else(|error| panic!("finding must re-encode: {error}"));
    let received = ReceivedHealthFinding::from_slice(&finding, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("finding must decode tolerantly: {error}"));
    assert_eq!(
        received.code().map(ToString::to_string).as_deref(),
        Some("DSP-1010")
    );
    assert!(received.protocol_issues().is_empty());
}
