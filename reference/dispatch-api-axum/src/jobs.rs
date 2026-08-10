//! Thin create and lookup handlers over the framework-neutral service.

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use dispatch_diagnostics::{
    DispatchCatalog, IdempotencyConflict, IdempotencyConflictEvidence, InternalError, JobNotFound,
    JobNotFoundEvidence, QueueUnavailable, ServiceTemporarilyUnavailable, ValidationFailed,
};
use dispatch_model::{Job, JobId};
use dispatch_service::{AdmissionRefusal, CreateJobOutcome};
use recourse::{
    diagnostic::{NoEvidence, PublicText},
    fault::PrivateReport,
    validation::{ParameterName, ValidationEvidence, Violation, ViolationReason, ViolationSource},
};
use recourse_axum::{HandlerResult, HttpFailure, ProblemContext};

use crate::{auth, input, state::ApiState};

const JOB_ID_PARAMETER: ParameterName = ParameterName::from_static("job_id");
const JOB_ID_DETAIL: PublicText =
    PublicText::from_static("Provide a canonical job_ prefixed ULID.");

pub(crate) async fn create(
    State(state): State<ApiState>,
    problems: ProblemContext<DispatchCatalog>,
    headers: HeaderMap,
    body: Bytes,
) -> HandlerResult<(StatusCode, Json<Job>)> {
    auth::require(&headers, &problems)?;
    let (key, request) = input::create_job(&headers, &body, &problems)?;
    let outcome = state.service.create_job(key, request).map_err(|error| {
        problems.fault::<InternalError>(
            NoEvidence,
            PrivateReport::new(error).context("operation", "create_job"),
        )
    })?;
    match outcome {
        CreateJobOutcome::Created(job) => Ok((StatusCode::CREATED, Json(job))),
        CreateJobOutcome::Replayed(job) => Ok((StatusCode::OK, Json(job))),
        CreateJobOutcome::Conflict { original_job_id } => Err(problems
            .problem::<IdempotencyConflict>(IdempotencyConflictEvidence { original_job_id })),
        CreateJobOutcome::Refused(AdmissionRefusal::QueueUnavailable {
            evidence,
            retry_after,
        }) => Err(problems.problem_with::<QueueUnavailable>(evidence, retry_after)),
        CreateJobOutcome::Refused(AdmissionRefusal::CapacityExhausted { retry_after }) => {
            Err(problems.problem_with::<ServiceTemporarilyUnavailable>(NoEvidence, retry_after))
        }
    }
}

pub(crate) async fn get(
    State(state): State<ApiState>,
    problems: ProblemContext<DispatchCatalog>,
    headers: HeaderMap,
    Path(raw_job_id): Path<String>,
) -> HandlerResult<Json<Job>> {
    auth::require(&headers, &problems)?;
    let job_id = JobId::new(raw_job_id).map_err(|_| invalid_job_id(&problems))?;
    let job = state.service.get_job(&job_id).map_err(|error| {
        problems.fault::<InternalError>(
            NoEvidence,
            PrivateReport::new(error).context("operation", "get_job"),
        )
    })?;
    job.map(Json).ok_or_else(|| {
        problems.problem::<JobNotFound>(JobNotFoundEvidence {
            job_id: job_id.clone(),
        })
    })
}

fn invalid_job_id(problems: &ProblemContext<DispatchCatalog>) -> HttpFailure {
    let violation = Violation {
        reason: ViolationReason::InvalidFormat,
        detail: JOB_ID_DETAIL,
        source: ViolationSource::Path {
            parameter: JOB_ID_PARAMETER,
        },
    };
    match ValidationEvidence::new(vec![violation]) {
        Ok(evidence) => problems.problem::<ValidationFailed>(evidence),
        Err(error) => problems.fault::<InternalError>(
            NoEvidence,
            PrivateReport::new(error).context("operation", "validate_job_id"),
        ),
    }
}
