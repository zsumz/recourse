//! Explicit failures from framework-neutral Dispatch state management.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use dispatch_model::{JobId, JobIdError, JobState};

/// Unexpected service failure suitable for a private Recourse report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchServiceError {
    /// Configured generator returned a malformed job identity.
    JobIdGeneration(JobIdError),
    /// Configured generator returned an identity already in use.
    DuplicateGeneratedId {
        /// Colliding public job identity.
        job_id: JobId,
    },
    /// Internal idempotency index no longer points to a stored job.
    RegistryInvariant,
    /// A worker referenced a job absent from the service registry.
    JobNotFound {
        /// Missing public job identity.
        job_id: JobId,
    },
    /// A worker tried to fail a job that was no longer accepted.
    JobNotAccepted {
        /// Public job identity.
        job_id: JobId,
        /// Current terminal state that rejected the transition.
        state: JobState,
    },
    /// A previous panic poisoned the in-memory reference store.
    RegistryPoisoned,
}

impl Display for DispatchServiceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::JobIdGeneration(error) => write!(formatter, "generate job ID: {error}"),
            Self::DuplicateGeneratedId { job_id } => {
                write!(formatter, "generated duplicate job ID {job_id}")
            }
            Self::RegistryInvariant => formatter.write_str("idempotency registry is inconsistent"),
            Self::JobNotFound { job_id } => write!(formatter, "job {job_id} does not exist"),
            Self::JobNotAccepted { job_id, state } => {
                write!(formatter, "job {job_id} cannot fail from state {state:?}")
            }
            Self::RegistryPoisoned => formatter.write_str("job registry lock is poisoned"),
        }
    }
}

impl Error for DispatchServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::JobIdGeneration(error) => Some(error),
            Self::DuplicateGeneratedId { .. }
            | Self::RegistryInvariant
            | Self::JobNotFound { .. }
            | Self::JobNotAccepted { .. }
            | Self::RegistryPoisoned => None,
        }
    }
}
