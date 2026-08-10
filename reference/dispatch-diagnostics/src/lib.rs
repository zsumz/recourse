//! Dispatch-owned evidence, diagnostic declarations, and catalog assembly.

mod catalog;
mod evidence;
mod health;
mod operation;
mod problem;
mod problem_set;

pub use catalog::{DispatchCatalog, catalog};
pub use evidence::{
    DispatchFailedEvidence, DispatchImpact, IdempotencyConflictEvidence, JobNotFoundEvidence,
    QueueUnavailableEvidence,
};
pub use health::QueueUnavailable;
pub use operation::DispatchFailed;
pub use problem::{
    AuthenticationRequired, IdempotencyConflict, InternalError, JobNotFound, MalformedRequest,
    ServiceTemporarilyUnavailable, UnsupportedMediaType, UnsupportedMethod, ValidationFailed,
};
pub use problem_set::{create_job_problems, get_job_problems};
