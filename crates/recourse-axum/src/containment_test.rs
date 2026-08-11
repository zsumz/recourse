//! Clone isolation and synchronous panic-containment regressions.

use std::{
    convert::Infallible,
    error::Error,
    fmt::{self, Display, Formatter},
    future::{Ready, ready},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
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
    observe::{FaultEvent, FaultReporter},
};

use super::{RecourseLayer, RequestIdGenerator, builder::RecourseLayerBuilder};
use tower::{Layer, Service, ServiceExt};

const CANARY: &str = "PRIVATE_CONTAINMENT_CANARY";

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "containment-test";
    const PREFIX: &'static str = "CNT";
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

fn builder() -> RecourseLayerBuilder<TestCatalog> {
    let catalog = Catalog::builder()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    RecourseLayer::builder(catalog).internal::<Internal>()
}

fn layer(reports: &Arc<Mutex<Vec<String>>>) -> RecourseLayer<TestCatalog> {
    builder()
        .fault_reporter(RecordingReporter(Arc::clone(reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"))
}

fn request() -> Request<Body> {
    Request::get("/test")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"))
}

#[derive(Debug)]
struct CanaryError;

impl Display for CanaryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(CANARY)
    }
}

impl Error for CanaryError {}

#[derive(Debug)]
struct CloneReadinessService {
    fail_readiness: bool,
    ready: bool,
}

impl Clone for CloneReadinessService {
    fn clone(&self) -> Self {
        Self {
            fail_readiness: false,
            ready: false,
        }
    }
}

impl Service<Request<Body>> for CloneReadinessService {
    type Response = Response;
    type Error = CanaryError;
    type Future = Ready<Result<Response, CanaryError>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.fail_readiness {
            Poll::Ready(Err(CanaryError))
        } else {
            self.ready = true;
            Poll::Ready(Ok(()))
        }
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        assert!(self.ready, "an unready inner clone was called");
        self.ready = false;
        ready(Ok(StatusCode::OK.into_response()))
    }
}

#[tokio::test]
async fn readiness_failures_belong_only_to_the_polled_clone() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let mut clone_a = layer(&reports).layer(CloneReadinessService {
        fail_readiness: true,
        ready: false,
    });
    let mut clone_b = clone_a.clone();
    let mut context = Context::from_waker(Waker::noop());

    assert!(matches!(
        clone_b.poll_ready(&mut context),
        Poll::Ready(Ok(()))
    ));
    assert!(matches!(
        clone_a.poll_ready(&mut context),
        Poll::Ready(Ok(()))
    ));
    let response_b = clone_b
        .call(request())
        .await
        .unwrap_or_else(|error| match error {});
    let response_a = clone_a
        .call(request())
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response_b.status(), StatusCode::OK);
    assert_eq!(response_a.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        reports
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len(),
        1
    );
}

#[derive(Debug, Default)]
struct FailOnceRequestIds(AtomicBool);

impl RequestIdGenerator for FailOnceRequestIds {
    fn generate(&self) -> Result<CorrelationId, CorrelationIdError> {
        if !self.0.swap(true, Ordering::Relaxed) {
            return CorrelationId::new("");
        }
        CorrelationId::new("request-after-preparation-failure")
    }
}

#[derive(Debug, Default)]
struct FailReadinessOnce(bool);

impl Service<Request<Body>> for FailReadinessOnce {
    type Response = Response;
    type Error = CanaryError;
    type Future = Ready<Result<Response, CanaryError>>;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if !self.0 {
            self.0 = true;
            return Poll::Ready(Err(CanaryError));
        }
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _request: Request<Body>) -> Self::Future {
        ready(Ok(StatusCode::OK.into_response()))
    }
}

#[tokio::test]
async fn preparation_failure_consumes_the_pending_readiness_failure() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let configured = builder()
        .request_ids(FailOnceRequestIds::default())
        .fault_reporter(RecordingReporter(Arc::clone(&reports)))
        .build()
        .unwrap_or_else(|error| panic!("test layer must build: {error}"));
    let mut service = configured.layer(FailReadinessOnce::default());
    let mut context = Context::from_waker(Waker::noop());

    assert!(matches!(
        service.poll_ready(&mut context),
        Poll::Ready(Ok(()))
    ));
    let failed = service
        .call(request())
        .await
        .unwrap_or_else(|error| match error {});
    assert_eq!(failed.status(), StatusCode::INTERNAL_SERVER_ERROR);

    assert!(matches!(
        service.poll_ready(&mut context),
        Poll::Ready(Ok(()))
    ));
    let healthy = service
        .call(request())
        .await
        .unwrap_or_else(|error| match error {});
    assert_eq!(healthy.status(), StatusCode::OK);
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
        panic!("{CANARY}")
    }
}

#[tokio::test]
async fn a_synchronous_service_panic_becomes_the_internal_problem() {
    let reports = Arc::new(Mutex::new(Vec::new()));
    let response = layer(&reports)
        .layer(SynchronousPanicService)
        .oneshot(request())
        .await
        .unwrap_or_else(|error| match error {});

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        reports
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[0]
            .contains(CANARY)
    );
}
