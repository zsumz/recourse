//! Exact HTTP error-family behavior through the Dispatch reference Router.

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
    response::Response,
};
use tower::ServiceExt;

fn request(method: Method, body: &'static str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri("/jobs")
        .header("authorization", "Bearer dispatch-demo")
        .header("content-type", "application/json")
        .header("idempotency-key", "protocol-test")
        .body(Body::from(body))
        .unwrap_or_else(|error| panic!("test request must build: {error}"))
}

async fn send(request: Request<Body>) -> Response {
    dispatch_api_axum::router()
        .unwrap_or_else(|error| panic!("test router must build: {error}"))
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {})
}

async fn problem(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 8192)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("Problem body must be JSON: {error}"))
}

#[tokio::test]
async fn missing_authentication_has_a_bearer_challenge() {
    let mut request = request(Method::POST, r#"{"destination":"west"}"#);
    request.headers_mut().remove("authorization");
    let response = send(request).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()["www-authenticate"],
        "Bearer realm=\"dispatch\""
    );
    assert_eq!(problem(response).await["code"], "DSP-1005");
}

#[tokio::test]
async fn media_syntax_and_semantics_have_distinct_problems() {
    let mut media_request = request(Method::POST, "not-json");
    media_request.headers_mut().insert(
        "content-type",
        "text/plain"
            .parse()
            .unwrap_or_else(|error| panic!("test content type must be valid: {error}")),
    );
    let media = send(media_request).await;
    assert_eq!(media.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(problem(media).await["code"], "DSP-1011");

    let malformed = send(request(Method::POST, "not-json")).await;
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(problem(malformed).await["code"], "DSP-1001");

    let mut semantic_request = request(Method::POST, r#"{"destination":""}"#);
    semantic_request.headers_mut().remove("idempotency-key");
    let semantic = send(semantic_request).await;
    assert_eq!(semantic.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let wire = problem(semantic).await;
    assert_eq!(wire["code"], "DSP-1002");
    assert_eq!(
        wire["evidence"]["violations"].as_array().map(Vec::len),
        Some(2)
    );
}

#[tokio::test]
async fn a_malformed_body_returns_the_frozen_canonical_problem() {
    let mut malformed = request(Method::POST, "not-json");
    malformed.headers_mut().insert(
        "x-request-id",
        "canonical-problem-test"
            .parse()
            .unwrap_or_else(|error| panic!("test correlation ID must be valid: {error}")),
    );
    let response = send(malformed).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    assert_eq!(response.headers()["x-request-id"], "canonical-problem-test");
    let body = to_bytes(response.into_body(), 8192)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    let fixture = include_bytes!("../../../conformance/wire/dispatch-problem.json");
    assert_eq!(
        body.as_ref(),
        fixture.strip_suffix(b"\n").unwrap_or(fixture)
    );
}

#[tokio::test]
async fn an_unsupported_media_type_body_is_canonical() {
    let mut media_request = request(Method::POST, "not-json");
    media_request.headers_mut().insert(
        "content-type",
        "text/plain"
            .parse()
            .unwrap_or_else(|error| panic!("test content type must be valid: {error}")),
    );
    let response = send(media_request).await;

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    let wire = problem(response).await;
    assert_eq!(wire["type"], "https://dispatch.invalid/problems/DSP-1011");
    assert_eq!(wire["title"], "Unsupported media type");
    assert_eq!(wire["status"], 415);
    assert_eq!(
        wire["detail"],
        "This operation accepts application/json requests."
    );
    assert_eq!(wire["code"], "DSP-1011");
    assert_eq!(wire["evidence"], serde_json::json!({}));
    assert_eq!(
        wire["suggestions"],
        serde_json::json!([
            "Encode the request body as JSON.",
            "Set Content-Type to application/json."
        ])
    );
    assert!(wire["instance"].as_str().is_some_and(|instance| {
        instance.starts_with("https://api.dispatch.invalid/problem-occurrences/")
    }));
}

#[tokio::test]
async fn semantic_violations_are_plural_and_typed() {
    let mut semantic_request = request(Method::POST, r#"{"destination":""}"#);
    semantic_request.headers_mut().remove("idempotency-key");
    let response = send(semantic_request).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let wire = problem(response).await;
    assert_eq!(wire["code"], "DSP-1002");
    assert_eq!(
        wire["evidence"]["violations"],
        serde_json::json!([
            {
                "reason": "out_of_range",
                "detail": "Provide a nonempty destination of at most 256 bytes.",
                "source": { "body": { "pointer": "/destination" } }
            },
            {
                "reason": "required",
                "detail": "Provide a visible-ASCII Idempotency-Key of at most 128 bytes.",
                "source": { "header": { "name": "idempotency-key" } }
            }
        ])
    );
}

#[tokio::test]
async fn unsupported_method_has_a_route_specific_allow_header() {
    let response = send(request(Method::PUT, "")).await;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(response.headers()["allow"], "POST");
    assert_eq!(problem(response).await["code"], "DSP-1006");
}
