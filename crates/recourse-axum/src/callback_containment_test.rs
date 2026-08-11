//! Request-scoping and telemetry callback panic tests.

use std::{
    convert::Infallible,
    future::Ready,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use http::{Request, StatusCode};
use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    fault::PrivateReport,
    http::{CorrelationId, CorrelationIdError, Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter, HttpObserver},
};
use tower::{Layer, Service, ServiceExt};

use super::{ProblemContext, RecourseLayer, RequestIdGenerator};

const CANARY: &str = "PRIVATE_CALLBACK_CANARY";

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "callback-containment-test";
    const PREFIX: &'static str = "CBK";
    const TYPE_BASE: &'static str = "https://axum.invalid/problems/";
}

enum Internal {}

impl DiagnosticType for Internal {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;
    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Internal error";
    const DETAIL: &'static str = "The request could not be completed.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Sanitized fallback.";
}

impl HttpProblemType for Internal {
    type Policy = Fixed<500>;
}

fn builder() -> crate::builder::RecourseLayerBuilder<TestCatalog> {
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    RecourseLayer::builder(catalog).internal::<Internal>()
}

fn request() -> Request<Body> {
    Request::get("/test")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"))
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
struct PanickingRequestIds;

impl RequestIdGenerator for PanickingRequestIds {
    fn generate(&self) -> Result<CorrelationId, CorrelationIdError> {
        panic!("{CANARY}")
    }
}

#[tokio::test]
async fn request_scope_callback_panics_become_internal_problems() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let generated_layer = builder()
        .request_ids(PanickingRequestIds)
        .fault_reporter(RecordingReporter(Arc::clone(&reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let instance_layer = builder()
        .instance_uri(|_| panic!("{CANARY}"))
        .fault_reporter(RecordingReporter(Arc::clone(&reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));

    let generated = generated_layer
        .layer(tower::service_fn(ok))
        .oneshot(request())
        .await
        .unwrap_or_else(|error| match error {});
    let instance = instance_layer
        .layer(tower::service_fn(ok))
        .oneshot(request())
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(generated.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(instance.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let reports = reports
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(reports.len(), 2);
    assert!(reports.iter().all(|report| report.contains(CANARY)));
}

async fn ok(_request: Request<Body>) -> Result<Response, Infallible> {
    Ok(StatusCode::OK.into_response())
}

#[derive(Debug, Clone, Copy)]
struct PanickingHooks;

impl HttpObserver for PanickingHooks {
    fn on_fault(&self, _event: &FaultEvent) {
        panic!("{CANARY}");
    }
}

impl FaultReporter for PanickingHooks {
    fn report_fault(&self, _event: &FaultEvent, _report: &PrivateReport) {
        panic!("{CANARY}");
    }
}

#[derive(Debug, Clone, Copy)]
struct CanaryError;

impl std::fmt::Display for CanaryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(CANARY)
    }
}

impl std::error::Error for CanaryError {}

#[derive(Debug, Clone, Copy)]
struct FaultingService;

impl Service<Request<Body>> for FaultingService {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let context = request
            .extensions()
            .get::<ProblemContext<TestCatalog>>()
            .unwrap_or_else(|| panic!("Recourse must install request context"));
        std::future::ready(Ok(context
            .internal_fault(PrivateReport::new(CanaryError))
            .into_response()))
    }
}

#[tokio::test]
async fn telemetry_panics_cannot_suppress_the_caller_response() {
    let layer = builder()
        .observer(PanickingHooks)
        .fault_reporter(PanickingHooks)
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let response = layer
        .layer(FaultingService)
        .oneshot(request())
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
