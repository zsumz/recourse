//! Router assembly using public catalog and adapter configuration APIs.

use axum::{
    Router,
    routing::{get, post},
};
use dispatch_diagnostics::{DispatchCatalog, InternalError, catalog};
use dispatch_service::{DispatchService, JobAdmission, QueueObservation};
use dispatch_worker::DispatchWorker;
use recourse::health::{HealthFindingId, HealthSeverity, ObservationTime};
use recourse_axum::RecourseLayer;
use time::OffsetDateTime;

use crate::{ApiBuildError, fault::FaultLog, health, jobs, method, state::ApiState};

/// Builds an empty in-memory Dispatch API reference application.
pub fn router() -> Result<Router, ApiBuildError> {
    router_with_admission(&JobAdmission::with_defaults(default_queue_health()?))
}

/// Builds Dispatch with an application-supplied admission policy.
pub fn router_with_admission(admission: &JobAdmission) -> Result<Router, ApiBuildError> {
    let catalog = catalog()?;
    let service = DispatchService::new(admission.clone());
    let worker = DispatchWorker::new();
    worker.publish_queue_health(&catalog, &service)?;
    let layer = RecourseLayer::<DispatchCatalog>::builder(catalog)
        .internal::<InternalError>()
        .instance_uri(|correlation_id| {
            format!("https://api.dispatch.invalid/problem-occurrences/{correlation_id}")
        })
        .fault_reporter(FaultLog::to_stderr())
        .build()?;
    Ok(Router::new()
        .route("/jobs", post(jobs::create))
        .route("/jobs/{job_id}", get(jobs::get))
        .route("/health", get(health::get))
        .method_not_allowed_fallback(method::unsupported)
        .with_state(ApiState { service, worker })
        .layer(layer))
}

fn default_queue_health() -> Result<QueueObservation, ApiBuildError> {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")?;
    let observed_at = ObservationTime::try_new(OffsetDateTime::now_utc())?;
    Ok(QueueObservation::new(
        finding_id,
        HealthSeverity::Degraded,
        observed_at,
        3,
    ))
}
