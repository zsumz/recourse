//! Replaceable canonical job-ID generation without framework coupling.

use dispatch_model::{JobId, JobIdError};

/// Generates one canonical Dispatch job identity.
pub trait JobIdGenerator: Send + Sync + 'static {
    /// Returns a validated `job_`-prefixed identity.
    fn generate(&self) -> Result<JobId, JobIdError>;
}

/// ULID-backed production job-ID generator.
#[derive(Debug, Clone, Copy, Default)]
pub struct UlidJobIds;

impl JobIdGenerator for UlidJobIds {
    fn generate(&self) -> Result<JobId, JobIdError> {
        JobId::new(format!("job_{}", ulid::Ulid::new()))
    }
}
