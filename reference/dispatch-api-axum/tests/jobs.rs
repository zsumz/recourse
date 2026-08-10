//! End-to-end create, replay, conflict, lookup, and missing-job behavior.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
    response::Response,
};
use dispatch_model::Job;
use tower::ServiceExt;

fn create_request(key: &str, destination: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/jobs")
        .header("authorization", "Bearer dispatch-demo")
        .header("content-type", "application/json")
        .header("idempotency-key", key)
        .header("x-request-id", "jobs-test-request")
        .body(Body::from(format!(r#"{{"destination":"{destination}"}}"#)))
        .unwrap_or_else(|error| panic!("test request must build: {error}"))
}

fn get_request(job_id: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(format!("/jobs/{job_id}"))
        .header("authorization", "Bearer dispatch-demo")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"))
}

async fn send(app: &Router, request: Request<Body>) -> Response {
    app.clone()
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {})
}

async fn json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 8192)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("response body must be JSON: {error}"))
}

#[tokio::test]
async fn create_replay_and_lookup_share_one_public_job() {
    let app = dispatch_api_axum::router()
        .unwrap_or_else(|error| panic!("test router must build: {error}"));
    let created = send(&app, create_request("same-key", "west")).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    assert_eq!(created.headers()["x-request-id"], "jobs-test-request");
    let created: Job = serde_json::from_value(json(created).await)
        .unwrap_or_else(|error| panic!("created job must decode: {error}"));

    let replay = send(&app, create_request("same-key", "west")).await;
    assert_eq!(replay.status(), StatusCode::OK);
    let replay: Job = serde_json::from_value(json(replay).await)
        .unwrap_or_else(|error| panic!("replayed job must decode: {error}"));
    assert_eq!(replay, created);

    let fetched = send(&app, get_request(created.id.as_str())).await;
    assert_eq!(fetched.status(), StatusCode::OK);
    let fetched: Job = serde_json::from_value(json(fetched).await)
        .unwrap_or_else(|error| panic!("fetched job must decode: {error}"));
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn conflicting_reuse_and_missing_lookup_are_typed_problems() {
    let app = dispatch_api_axum::router()
        .unwrap_or_else(|error| panic!("test router must build: {error}"));
    let created = send(&app, create_request("conflict-key", "west")).await;
    let created: Job = serde_json::from_value(json(created).await)
        .unwrap_or_else(|error| panic!("created job must decode: {error}"));

    let conflict = send(&app, create_request("conflict-key", "east")).await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let conflict = json(conflict).await;
    assert_eq!(conflict["code"], "DSP-1004");
    assert_eq!(conflict["evidence"]["original_job_id"], created.id.as_str());

    let missing = send(&app, get_request("job_01K00000000000000000000000")).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(json(missing).await["code"], "DSP-1003");
}

#[tokio::test]
async fn a_missing_job_carries_typed_job_identifier_evidence() {
    let app = dispatch_api_axum::router()
        .unwrap_or_else(|error| panic!("test router must build: {error}"));
    let missing = send(&app, get_request("job_01K00000000000000000000000")).await;

    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let wire = json(missing).await;
    assert_eq!(wire["code"], "DSP-1003");
    assert_eq!(wire["type"], "https://dispatch.invalid/problems/DSP-1003");
    assert_eq!(
        wire["evidence"],
        serde_json::json!({ "job_id": "job_01K00000000000000000000000" })
    );
}
