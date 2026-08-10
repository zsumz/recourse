//! Application-owned aggregate health resource over governed findings.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use axum::{Json, extract::State};
use dispatch_diagnostics::{DispatchCatalog, InternalError};
use recourse::{diagnostic::NoEvidence, fault::PrivateReport, health::HealthSeverity};
use recourse_axum::{HandlerResult, ProblemContext};
use serde::Serialize;
use serde_json::Value;

use crate::state::ApiState;

#[derive(Debug, Serialize)]
pub(crate) struct HealthResource {
    status: HealthStatus,
    observed_at: String,
    findings: Vec<Value>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum HealthStatus {
    Degraded,
    Unhealthy,
}

pub(crate) async fn get(
    State(state): State<ApiState>,
    problems: ProblemContext<DispatchCatalog>,
) -> HandlerResult<Json<HealthResource>> {
    let published = state
        .worker
        .published_queue_health()
        .map_err(|error| {
            problems.fault::<InternalError>(
                NoEvidence,
                PrivateReport::new(error).context("operation", "read_health_publication"),
            )
        })?
        .ok_or_else(|| {
            problems.fault::<InternalError>(
                NoEvidence,
                PrivateReport::new(MissingPublication)
                    .context("operation", "read_health_publication"),
            )
        })?;
    let observation = published.observation();
    Ok(Json(HealthResource {
        status: status(observation.severity()),
        observed_at: observation.observed_at().as_str().to_owned(),
        findings: vec![published.body().clone()],
    }))
}

const fn status(severity: HealthSeverity) -> HealthStatus {
    match severity {
        HealthSeverity::Degraded => HealthStatus::Degraded,
        HealthSeverity::Unhealthy => HealthStatus::Unhealthy,
    }
}

#[derive(Debug, Clone, Copy)]
struct MissingPublication;

impl Display for MissingPublication {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("worker has not published queue health")
    }
}

impl Error for MissingPublication {}
