//! End-to-end proof that private fault material never reaches HTTP callers.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    routing::get,
};
use http::{Request, StatusCode};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    fault::PrivateReport,
    http::{Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter, HttpObserver},
};
use tower::ServiceExt;

use super::{HandlerResult, ProblemContext, RecourseLayer};

const PRIVATE_CANARY: &str = "PRIVATE_DATABASE_TOKEN_7cf4";

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "axum-fault-test";
    const PREFIX: &'static str = "AFT";
    const TYPE_BASE: &'static str = "https://axum.invalid/problems/";
}

#[derive(Debug)]
enum Internal {}

impl DiagnosticType for Internal {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Internal error";
    const DETAIL: &'static str = "The service could not complete the request.";
    const SUGGESTIONS: &'static [&'static str] = &["Retry the request later."];
    const DOCS: &'static str = "Sanitized internal failure.";
}

impl HttpProblemType for Internal {
    type Policy = Fixed<500>;
}

#[derive(Debug, Clone)]
struct RecordingPorts(Arc<Mutex<RecordingState>>);

#[derive(Debug, Default)]
struct RecordingState {
    fault_events: usize,
    reports: Vec<String>,
}

impl HttpObserver for RecordingPorts {
    fn on_fault(&self, _event: &FaultEvent) {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.fault_events += 1;
    }
}

impl FaultReporter for RecordingPorts {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.reports.push(report.to_string());
    }
}

#[derive(Debug)]
struct CanaryError;

impl Display for CanaryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(PRIVATE_CANARY)
    }
}

impl Error for CanaryError {}

async fn fail(problems: ProblemContext<TestCatalog>) -> HandlerResult<&'static str> {
    Err(problems.fault::<Internal>(
        NoEvidence,
        PrivateReport::new(CanaryError).context("query", PRIVATE_CANARY),
    ))
}

async fn panic_before_response() -> &'static str {
    panic!("{PRIVATE_CANARY}");
}

#[tokio::test]
async fn caller_sees_sanitized_problem_while_reporter_receives_canary() {
    let state = Arc::new(Mutex::new(RecordingState::default()));
    let ports = RecordingPorts(Arc::clone(&state));
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let layer = RecourseLayer::builder(catalog)
        .internal::<Internal>()
        .observer(ports.clone())
        .fault_reporter(ports)
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let request = Request::builder()
        .uri("/fail")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = Router::new()
        .route("/fail", get(fail))
        .layer(layer)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    assert!(!String::from_utf8_lossy(&body).contains(PRIVATE_CANARY));
    let state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.fault_events, 1);
    assert_eq!(state.reports.len(), 1);
    assert!(state.reports[0].contains(PRIVATE_CANARY));
}

#[tokio::test]
async fn recovered_panic_uses_internal_problem_and_private_reporter() {
    let state = Arc::new(Mutex::new(RecordingState::default()));
    let ports = RecordingPorts(Arc::clone(&state));
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let layer = RecourseLayer::builder(catalog)
        .internal::<Internal>()
        .observer(ports.clone())
        .fault_reporter(ports)
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let request = Request::builder()
        .uri("/panic")
        .header("x-request-id", "panic-request")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = Router::new()
        .route("/panic", get(panic_before_response))
        .layer(layer)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(response.headers()["x-request-id"], "panic-request");
    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    assert!(!String::from_utf8_lossy(&body).contains(PRIVATE_CANARY));
    let state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(state.fault_events, 1);
    assert!(state.reports[0].contains(PRIVATE_CANARY));
}
