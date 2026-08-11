//! Unexpected Dispatch failures stay sanitized in public, private to operators.

use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Method, Request, StatusCode},
    response::Response,
    routing::{get, post},
};
use dispatch_diagnostics::{DispatchCatalog, InternalError, catalog};
use dispatch_model::{CreateJobRequest, Destination, IdempotencyKey, JobId, JobIdError};
use dispatch_service::{DispatchService, JobAdmission, JobIdGenerator, QueueObservation};
use recourse::{
    diagnostic::NoEvidence,
    fault::PrivateReport,
    health::{HealthFindingId, HealthSeverity, ObservationTime},
    observe::{FaultEvent, FaultReporter, HttpObserver},
};
use recourse_axum::{HandlerResult, ProblemContext, RecourseLayer};
use tower::ServiceExt;

const PRIVATE_STORE: &str = "postgres://dispatch:PRIVATE_STORAGE_TOKEN_9ba2@jobs.internal";
const PRIVATE_PANIC: &str = "PRIVATE_PANIC_DETAIL_4c81";
const SOURCE_SUMMARY: &str = "generate job ID";

/// Storage generator that always rejects the identity it was asked to mint.
#[derive(Debug, Clone, Copy)]
struct UnusableJobIds;

impl JobIdGenerator for UnusableJobIds {
    fn generate(&self) -> Result<JobId, JobIdError> {
        JobId::new("01K00000000000000000000000")
    }
}

#[derive(Debug, Clone)]
struct FailingStorage {
    service: DispatchService<UnusableJobIds>,
    key: IdempotencyKey,
    request: CreateJobRequest,
}

#[derive(Debug, Default)]
struct Recorded {
    faults: usize,
    reports: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct RecordingPorts(Arc<Mutex<Recorded>>);

impl RecordingPorts {
    fn recorded(&self) -> (usize, Vec<String>) {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        (state.faults, state.reports.clone())
    }
}

impl HttpObserver for RecordingPorts {
    fn on_fault(&self, _event: &FaultEvent) {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).faults += 1;
    }
}

impl FaultReporter for RecordingPorts {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .reports
            .push(report.to_string());
    }
}

fn admitting() -> JobAdmission {
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

fn failing_storage() -> FailingStorage {
    let key = IdempotencyKey::new("fault-fixture")
        .unwrap_or_else(|error| panic!("fixture key must be valid: {error}"));
    let destination = Destination::new("west")
        .unwrap_or_else(|error| panic!("fixture destination must be valid: {error}"));
    FailingStorage {
        service: DispatchService::with_generator(UnusableJobIds, admitting()),
        key,
        request: CreateJobRequest { destination },
    }
}

fn app(ports: RecordingPorts) -> Router {
    let catalog = catalog().unwrap_or_else(|error| panic!("Dispatch catalog must build: {error}"));
    let layer = RecourseLayer::<DispatchCatalog>::builder(catalog)
        .internal::<InternalError>()
        .instance_uri(|correlation_id| {
            format!("https://api.dispatch.invalid/problem-occurrences/{correlation_id}")
        })
        .observer(ports.clone())
        .fault_reporter(ports)
        .build()
        .unwrap_or_else(|error| panic!("Dispatch layer must build: {error}"));
    Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/panicking", get(panic_before_response))
        .with_state(failing_storage())
        .layer(layer)
}

async fn create_job(
    State(storage): State<FailingStorage>,
    problems: ProblemContext<DispatchCatalog>,
) -> HandlerResult<&'static str> {
    storage
        .service
        .create_job(storage.key, storage.request)
        .map_err(|error| {
            problems.fault::<InternalError>(
                NoEvidence,
                PrivateReport::new(error)
                    .context("operation", "create_job")
                    .context("store", PRIVATE_STORE),
            )
        })?;
    Ok("accepted")
}

async fn panic_before_response() -> &'static str {
    panic!("{PRIVATE_PANIC}");
}

async fn send(ports: RecordingPorts, method: Method, uri: &str) -> Response {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-request-id", "fault-test-request")
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("test request must build: {error}"));
    app(ports)
        .oneshot(request)
        .await
        .unwrap_or_else(|error| match error {})
}

async fn sanitized_internal_body(response: Response) -> String {
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );
    assert_eq!(response.headers()["x-request-id"], "fault-test-request");
    let body = to_bytes(response.into_body(), 8192)
        .await
        .unwrap_or_else(|error| panic!("Problem body must be readable: {error}"));
    let problem: serde_json::Value = serde_json::from_slice(&body)
        .unwrap_or_else(|error| panic!("Problem body must be JSON: {error}"));
    assert_eq!(problem["code"], "DSP-1008");
    assert_eq!(problem["status"], 500);
    assert_eq!(problem["title"], "Internal error");
    assert_eq!(problem["evidence"], serde_json::json!({}));
    assert_eq!(
        problem["instance"],
        "https://api.dispatch.invalid/problem-occurrences/fault-test-request"
    );
    String::from_utf8_lossy(&body).into_owned()
}

#[tokio::test]
async fn an_unexpected_storage_failure_is_public_500_and_private_source_report() {
    let ports = RecordingPorts::default();
    let response = send(ports.clone(), Method::POST, "/jobs").await;
    let body = sanitized_internal_body(response).await;

    assert!(!body.contains(PRIVATE_STORE));
    assert!(!body.contains(SOURCE_SUMMARY));
    let (faults, reports) = ports.recorded();
    assert_eq!(faults, 1);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].contains(SOURCE_SUMMARY));
    assert!(reports[0].contains("[operation=create_job]"));
    assert!(reports[0].contains(PRIVATE_STORE));
}

#[tokio::test]
async fn a_panic_before_response_start_is_public_500_and_private_panic_report() {
    let ports = RecordingPorts::default();
    let response = send(ports.clone(), Method::GET, "/jobs/panicking").await;
    let body = sanitized_internal_body(response).await;

    assert!(!body.contains(PRIVATE_PANIC));
    let (faults, reports) = ports.recorded();
    assert_eq!(faults, 1);
    assert_eq!(reports.len(), 1);
    assert!(reports[0].contains("request boundary panicked"));
    assert!(reports[0].contains("[recourse_stage=request_service_panic]"));
    assert!(reports[0].contains(PRIVATE_PANIC));
}
