//! Transient Dispatch dependency outages publish a governed retry delay.
//!
//! Both refusals come from the real router: the framework-neutral service
//! owns the admission decision and the minimum delay, and the handler only
//! translates the refusal into a header-aware Problem.

use std::time::Duration;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::Response,
};
use dispatch_service::{JobAdmission, QueueObservation};
use recourse::health::{HealthFindingId, HealthSeverity, ObservationTime};
use tower::ServiceExt;

const MINIMUM_DELAY: Duration = Duration::from_secs(30);

fn admission(severity: HealthSeverity, capacity: usize) -> JobAdmission {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("fixture finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture observation time must be valid: {error}"));
    JobAdmission::new(
        QueueObservation::new(finding_id, severity, observed_at, 3),
        capacity,
        MINIMUM_DELAY,
    )
}

fn app(admission: &JobAdmission) -> Router {
    dispatch_api_axum::router_with_admission(admission)
        .unwrap_or_else(|error| panic!("Dispatch router must build: {error}"))
}

async fn create_job(admission: &JobAdmission) -> Response {
    let request = Request::post("/jobs")
        .header("authorization", "Bearer dispatch-demo")
        .header("content-type", "application/json")
        .header("idempotency-key", "outage-fixture")
        .body(Body::from(r#"{"destination":"west"}"#))
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    app(admission)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {})
}

async fn problem(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 8192)
        .await
        .unwrap_or_else(|error| panic!("Problem body must be readable: {error}"));
    serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("Problem body must be JSON: {error}"))
}

#[tokio::test]
async fn an_unreachable_queue_is_503_with_the_minimum_retry_delay() {
    let response = create_job(&admission(HealthSeverity::Unhealthy, 16)).await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["retry-after"], "30");
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    let wire = problem(response).await;
    assert_eq!(wire["code"], "DSP-1010");
    assert_eq!(wire["status"], 503);
    assert_eq!(wire["evidence"]["consecutive_failures"], 3);
}

#[tokio::test]
async fn exhausted_capacity_is_503_with_the_minimum_retry_delay() {
    let response = create_job(&admission(HealthSeverity::Degraded, 0)).await;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["retry-after"], "30");
    let wire = problem(response).await;
    assert_eq!(wire["code"], "DSP-1007");
    assert_eq!(wire["status"], 503);
    assert_eq!(wire["evidence"], serde_json::json!({}));
}
