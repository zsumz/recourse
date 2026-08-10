//! Thread-safe framework-neutral job creation and lookup service.

use std::sync::{Arc, Mutex, MutexGuard};

use dispatch_model::{CreateJobRequest, IdempotencyKey, Job, JobId};

use crate::{
    AdmissionRefusal, DispatchServiceError, JobAdmission, JobIdGenerator, UlidJobIds,
    registry::JobRegistry,
};

/// Idempotency-aware result of one create operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateJobOutcome {
    /// A new job was accepted.
    Created(Job),
    /// The same request was replayed under the same key.
    Replayed(Job),
    /// The key already identifies a job created from other inputs.
    Conflict {
        /// Job bound by the first request.
        original_job_id: JobId,
    },
    /// A transient dependency condition blocked new work.
    Refused(AdmissionRefusal),
}

/// Cloneable application service independent of HTTP and async runtimes.
#[derive(Debug)]
pub struct DispatchService<G: JobIdGenerator = UlidJobIds> {
    inner: Arc<ServiceInner<G>>,
}

#[derive(Debug)]
struct ServiceInner<G: JobIdGenerator> {
    generator: G,
    admission: JobAdmission,
    registry: Mutex<JobRegistry>,
}

impl DispatchService<UlidJobIds> {
    /// Creates an empty service with ULID job identities.
    pub fn new(admission: JobAdmission) -> Self {
        Self::with_generator(UlidJobIds, admission)
    }
}

impl<G: JobIdGenerator> DispatchService<G> {
    /// Creates an empty service with an application-selected ID generator.
    pub fn with_generator(generator: G, admission: JobAdmission) -> Self {
        Self {
            inner: Arc::new(ServiceInner {
                generator,
                admission,
                registry: Mutex::new(JobRegistry::default()),
            }),
        }
    }

    /// Policy this service admits or transiently refuses new work against.
    pub fn admission(&self) -> &JobAdmission {
        &self.inner.admission
    }

    /// Accepts, replays, refuses, or rejects one idempotent job creation.
    pub fn create_job(
        &self,
        key: IdempotencyKey,
        request: CreateJobRequest,
    ) -> Result<CreateJobOutcome, DispatchServiceError> {
        let mut registry = self.lock_registry()?;
        // A replay or conflict resolves an existing job, so admission only
        // gates work that would enter the queue for the first time.
        if let Some(outcome) = registry.existing_outcome(&key, &request)? {
            return Ok(outcome);
        }
        if let Some(refusal) = self.inner.admission.refusal(registry.accepted_backlog()) {
            return Ok(CreateJobOutcome::Refused(refusal));
        }
        let job_id = self
            .inner
            .generator
            .generate()
            .map_err(DispatchServiceError::JobIdGeneration)?;
        registry.create(key, request, job_id)
    }

    /// Returns one public job when it exists.
    pub fn get_job(&self, job_id: &JobId) -> Result<Option<Job>, DispatchServiceError> {
        Ok(self.lock_registry()?.get(job_id))
    }

    /// Transitions one accepted job to its terminal failed state.
    pub fn mark_failed(&self, job_id: &JobId) -> Result<Job, DispatchServiceError> {
        self.lock_registry()?.mark_failed(job_id)
    }

    fn lock_registry(&self) -> Result<MutexGuard<'_, JobRegistry>, DispatchServiceError> {
        self.inner
            .registry
            .lock()
            .map_err(|_| DispatchServiceError::RegistryPoisoned)
    }
}

impl<G: JobIdGenerator> Clone for DispatchService<G> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
