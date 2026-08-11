//! Fallible Tower service conversion and private-reporting tests.

use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Display, Formatter},
    future::{Ready, ready},
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use axum::{
    body::{Body, to_bytes},
    response::{IntoResponse, Response},
};
use http::{Request, StatusCode};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    fault::PrivateReport,
    http::{Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter},
};
use tower::{Layer, Service, ServiceExt};

use super::RecourseLayer;

const PRIVATE_CANARY: &str = "PRIVATE_SERVICE_FAILURE_58ac";

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "service-error-test";
    const PREFIX: &'static str = "SET";
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
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Sanitized internal failure.";
}

impl HttpProblemType for Internal {
    type Policy = Fixed<500>;
}

#[derive(Debug, Clone)]
struct RecordingReporter(Arc<Mutex<Vec<String>>>);

impl FaultReporter for RecordingReporter {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(report.to_string());
    }
}

#[derive(Debug, Clone, Copy)]
struct FailingService;

impl Service<Request<Body>> for FailingService {
    type Response = Response;
    type Error = CanaryError;
    type Future = Ready<Result<Response, CanaryError>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        ready(Err(CanaryError))
    }
}

#[derive(Debug, Clone, Copy)]
struct ReadinessFailingService;

impl Service<Request<Body>> for ReadinessFailingService {
    type Response = Response;
    type Error = CanaryError;
    type Future = Ready<Result<Response, CanaryError>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err(CanaryError))
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        ready(Ok(StatusCode::OK.into_response()))
    }
}

#[derive(Debug, Clone, Copy)]
struct SynchronousPanicService;

impl Service<Request<Body>> for SynchronousPanicService {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        panic!("{PRIVATE_CANARY}")
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

#[tokio::test]
async fn service_errors_become_sanitized_internal_problems() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let layer = RecourseLayer::builder(catalog)
        .internal::<Internal>()
        .fault_reporter(RecordingReporter(Arc::clone(&reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let request = Request::builder()
        .uri("/fail")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = layer
        .layer(FailingService)
        .oneshot(request)
        .await
        .unwrap_or_else(|error: Infallible| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), 4096)
        .await
        .unwrap_or_else(|error| panic!("test body must be readable: {error}"));
    assert!(!String::from_utf8_lossy(&body).contains(PRIVATE_CANARY));
    let reports = reports
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].contains(PRIVATE_CANARY));
}

#[tokio::test]
async fn readiness_errors_are_held_for_the_next_scoped_request() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let layer = RecourseLayer::builder(catalog)
        .internal::<Internal>()
        .fault_reporter(RecordingReporter(Arc::clone(&reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let request = Request::builder()
        .uri("/unready")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    let response = layer
        .layer(ReadinessFailingService)
        .oneshot(request)
        .await
        .unwrap_or_else(|error: Infallible| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let reports = reports
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].contains(PRIVATE_CANARY));
}

#[tokio::test]
async fn synchronous_service_panics_become_internal_problems() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let layer = RecourseLayer::builder(catalog)
        .internal::<Internal>()
        .fault_reporter(RecordingReporter(Arc::clone(&reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let response = layer
        .layer(SynchronousPanicService)
        .oneshot(Request::new(Body::empty()))
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        reports
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[0]
            .contains(PRIVATE_CANARY)
    );
}
