//! Recourse-independent request, response, job, and identifier types.

mod destination;
mod idempotency_key;
mod job;
mod job_id;

pub use destination::{Destination, DestinationError};
pub use idempotency_key::{IdempotencyKey, IdempotencyKeyError};
pub use job::{CreateJobRequest, Job, JobState};
pub use job_id::{JobId, JobIdError};

#[cfg(test)]
mod destination_test;
#[cfg(test)]
mod idempotency_key_test;
#[cfg(test)]
mod job_id_test;
#[cfg(test)]
mod job_test;
