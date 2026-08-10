//! In-memory job and idempotency indexes with explicit invariants.

use std::collections::BTreeMap;

use dispatch_model::{CreateJobRequest, IdempotencyKey, Job, JobId, JobState};

use crate::{CreateJobOutcome, DispatchServiceError};

#[derive(Debug, Default)]
pub(crate) struct JobRegistry {
    jobs: BTreeMap<JobId, Job>,
    idempotency: BTreeMap<IdempotencyKey, JobId>,
    accepted: usize,
}

impl JobRegistry {
    /// Jobs still awaiting a worker, which is what admission capacity bounds.
    pub(crate) const fn accepted_backlog(&self) -> usize {
        self.accepted
    }

    pub(crate) fn existing_outcome(
        &self,
        key: &IdempotencyKey,
        request: &CreateJobRequest,
    ) -> Result<Option<CreateJobOutcome>, DispatchServiceError> {
        let Some(job_id) = self.idempotency.get(key) else {
            return Ok(None);
        };
        let job = self
            .jobs
            .get(job_id)
            .ok_or(DispatchServiceError::RegistryInvariant)?;
        if job.destination == request.destination {
            return Ok(Some(CreateJobOutcome::Replayed(job.clone())));
        }
        Ok(Some(CreateJobOutcome::Conflict {
            original_job_id: job.id.clone(),
        }))
    }

    pub(crate) fn create(
        &mut self,
        key: IdempotencyKey,
        request: CreateJobRequest,
        job_id: JobId,
    ) -> Result<CreateJobOutcome, DispatchServiceError> {
        if self.jobs.contains_key(&job_id) {
            return Err(DispatchServiceError::DuplicateGeneratedId { job_id });
        }
        let job = Job {
            id: job_id.clone(),
            destination: request.destination,
            state: JobState::Accepted,
        };
        self.idempotency.insert(key, job_id.clone());
        self.jobs.insert(job_id, job.clone());
        self.accepted = self.accepted.saturating_add(1);
        Ok(CreateJobOutcome::Created(job))
    }

    pub(crate) fn get(&self, job_id: &JobId) -> Option<Job> {
        self.jobs.get(job_id).cloned()
    }

    pub(crate) fn mark_failed(&mut self, job_id: &JobId) -> Result<Job, DispatchServiceError> {
        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| DispatchServiceError::JobNotFound {
                job_id: job_id.clone(),
            })?;
        if job.state != JobState::Accepted {
            return Err(DispatchServiceError::JobNotAccepted {
                job_id: job_id.clone(),
                state: job.state,
            });
        }
        job.state = JobState::Failed;
        let job = job.clone();
        self.accepted = self.accepted.saturating_sub(1);
        Ok(job)
    }
}
