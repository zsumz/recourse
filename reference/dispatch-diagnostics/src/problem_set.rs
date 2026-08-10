//! Stable Dispatch API operation declarations over HTTP diagnostics.

use recourse::catalog::ProblemSet;

use crate::{
    AuthenticationRequired, DispatchCatalog, IdempotencyConflict, InternalError, JobNotFound,
    MalformedRequest, QueueUnavailable, ServiceTemporarilyUnavailable, UnsupportedMediaType,
    ValidationFailed,
};

/// Problems the create-job operation declares as part of its API contract.
pub fn create_job_problems() -> ProblemSet<DispatchCatalog> {
    ProblemSet::builder("createJob")
        .include::<MalformedRequest>()
        .include::<UnsupportedMediaType>()
        .include::<ValidationFailed>()
        .include::<AuthenticationRequired>()
        .include::<IdempotencyConflict>()
        .include::<QueueUnavailable>()
        .include::<ServiceTemporarilyUnavailable>()
        .include::<InternalError>()
        .build()
}

/// Problems the get-job operation declares as part of its API contract.
pub fn get_job_problems() -> ProblemSet<DispatchCatalog> {
    ProblemSet::builder("getJob")
        .include::<ValidationFailed>()
        .include::<JobNotFound>()
        .include::<AuthenticationRequired>()
        .include::<InternalError>()
        .build()
}
