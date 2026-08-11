//! End-to-end Axum response and request-correlation tests.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    routing::get,
};
use http::{Request, StatusCode, header::CONTENT_TYPE};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    fault::PrivateReport,
    http::{CorrelationId, CorrelationIdError, Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter},
};
use tower::ServiceExt;

use super::{HandlerResult, ProblemContext, RecourseLayer, RequestIdGenerator};

#[test]
fn concrete_handler_failure_is_pointer_sized() {
    assert_eq!(
        std::mem::size_of::<super::HttpFailure>(),
        std::mem::size_of::<usize>()
    );
}

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "axum-layer-test";
    const PREFIX: &'static str = "ALT";
    const TYPE_BASE: &'static str = "https://axum.invalid/problems/";
}

#[derive(Debug)]
enum Missing {}

impl DiagnosticType for Missing {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Missing";
    const DETAIL: &'static str = "The resource is missing.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the resource identifier."];
    const DOCS: &'static str = "Missing resource.";
}

impl HttpProblemType for Missing {
    type Policy = Fixed<404>;
}

#[derive(Debug)]
enum Internal {}

impl DiagnosticType for Internal {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(2);
    const TITLE: &'static str = "Internal error";
    const DETAIL: &'static str = "The service could not complete the request.";
    const SUGGESTIONS: &'static [&'static str] = &["Retry the request later."];
    const DOCS: &'static str = "Sanitized internal failure.";
}

impl HttpProblemType for Internal {
    type Policy = Fixed<500>;
}

#[derive(Debug)]
struct FixedRequestId(&'static str);

impl RequestIdGenerator for FixedRequestId {
    fn generate(&self) -> Result<CorrelationId, CorrelationIdError> {
        CorrelationId::new(self.0)
    }
}

fn catalog() -> Catalog<TestCatalog> {
    Catalog::builder()
        .problem::<Missing>()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"))
}

#[derive(Debug, Clone, Default)]
struct CountingReports(Arc<AtomicUsize>);

impl CountingReports {
    fn count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

impl FaultReporter for CountingReports {
    fn report_fault(&self, _event: &FaultEvent, _report: &PrivateReport) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

fn app(reports: CountingReports) -> Router {
    let layer = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .request_ids(FixedRequestId("generated-request"))
        .instance_uri(|id| {
            format!(
                "https://api.invalid/problem-occurrences/{}",
                id.to_uri_path_segment()
            )
        })
        .fault_reporter(reports)
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    Router::new()
        .route("/resources/{id}", get(missing))
        .route("/ok", get(ok))
        .layer(layer)
}

async fn missing(problems: ProblemContext<TestCatalog>) -> HandlerResult<&'static str> {
    Err(problems.problem::<Missing>(NoEvidence))
}

async fn ok() -> &'static str {
    "ok"
}

#[tokio::test]
async fn problem_translation_preserves_canonical_wire_and_incoming_id() {
    let reports = CountingReports::default();
    let request = Request::builder()
        .uri("/resources/42")
        .header("x-request-id", "caller-request")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = app(reports.clone())
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(reports.count(), 0);
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/problem+json");
    assert_eq!(response.headers()["x-request-id"], "caller-request");
    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    let wire: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("Problem body must be JSON: {error}"));
    assert_eq!(wire["code"], "ALT-1");
    assert_eq!(wire["status"], 404);
    assert_eq!(
        wire["instance"],
        "https://api.invalid/problem-occurrences/caller-request"
    );
}

#[tokio::test]
async fn invalid_incoming_ids_are_replaced_and_every_response_echoes() {
    let reports = CountingReports::default();
    let oversized = "x".repeat(129);
    let request = Request::builder()
        .uri("/ok")
        .header("x-request-id", oversized)
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = app(reports.clone())
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(reports.count(), 0);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-request-id"], "generated-request");
}

#[tokio::test]
async fn default_instances_encode_hostile_request_ids_as_one_path_segment() {
    let reports = CountingReports::default();
    let layer = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .fault_reporter(reports)
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let request = Request::builder()
        .uri("/resources/42")
        .header("x-request-id", "../job?attempt#2%2F")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = Router::new()
        .route("/resources/{id}", get(missing))
        .layer(layer)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    let wire: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("Problem body must be JSON: {error}"));
    assert_eq!(
        wire["instance"],
        "/problem-occurrences/%2E%2E%2Fjob%3Fattempt%232%252F"
    );
}
