//! Idempotency, admission, lookup, and generator-invariant service tests.

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use dispatch_model::{CreateJobRequest, Destination, IdempotencyKey, JobId, JobIdError, JobState};
use recourse::health::{HealthFindingId, HealthSeverity, ObservationTime};

use super::{
    AdmissionRefusal, CreateJobOutcome, DispatchService, DispatchServiceError, JobAdmission,
    JobIdGenerator, QueueObservation,
};

fn admission(severity: HealthSeverity, capacity: usize) -> JobAdmission {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("test finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("test observation time must be valid: {error}"));
    JobAdmission::new(
        QueueObservation::new(finding_id, severity, observed_at, 3),
        capacity,
        Duration::from_secs(30),
    )
}

fn service<G: JobIdGenerator>(generator: G) -> DispatchService<G> {
    DispatchService::with_generator(generator, admission(HealthSeverity::Degraded, 16))
}

#[derive(Debug, Default)]
struct SequenceIds(AtomicUsize);

impl JobIdGenerator for SequenceIds {
    fn generate(&self) -> Result<JobId, JobIdError> {
        let suffix = match self.0.fetch_add(1, Ordering::Relaxed) {
            0 => "01K00000000000000000000000",
            _ => "01K00000000000000000000001",
        };
        JobId::new(format!("job_{suffix}"))
    }
}

fn request(destination: &str) -> CreateJobRequest {
    CreateJobRequest {
        destination: Destination::new(destination)
            .unwrap_or_else(|error| panic!("test destination must be valid: {error}")),
    }
}

fn key() -> IdempotencyKey {
    IdempotencyKey::new("create-job-test")
        .unwrap_or_else(|error| panic!("test key must be valid: {error}"))
}

#[test]
fn creation_replay_conflict_and_lookup_are_distinct() {
    let service = service(SequenceIds::default());
    let created = service
        .create_job(key(), request("west"))
        .unwrap_or_else(|error| panic!("first create must succeed: {error}"));
    let CreateJobOutcome::Created(job) = created else {
        panic!("first create must create a job");
    };

    let replay = service
        .create_job(key(), request("west"))
        .unwrap_or_else(|error| panic!("exact replay must succeed: {error}"));
    assert!(matches!(replay, CreateJobOutcome::Replayed(ref value) if value == &job));
    let conflict = service
        .create_job(key(), request("east"))
        .unwrap_or_else(|error| panic!("semantic conflict is an outcome: {error}"));
    assert!(matches!(
        conflict,
        CreateJobOutcome::Conflict { original_job_id } if original_job_id == job.id
    ));
    assert_eq!(service.get_job(&job.id).ok().flatten(), Some(job));
}

#[derive(Debug)]
struct DuplicateId;

impl JobIdGenerator for DuplicateId {
    fn generate(&self) -> Result<JobId, JobIdError> {
        JobId::new("job_01K00000000000000000000000")
    }
}

#[test]
fn duplicate_generated_identity_is_an_explicit_private_failure() {
    let service = service(DuplicateId);
    assert!(service.create_job(key(), request("west")).is_ok());
    let other_key = IdempotencyKey::new("other-key")
        .unwrap_or_else(|error| panic!("test key must be valid: {error}"));
    let error = service.create_job(other_key, request("east"));

    assert!(matches!(
        error,
        Err(DispatchServiceError::DuplicateGeneratedId { .. })
    ));
}

#[test]
fn only_an_accepted_job_can_transition_to_failed() {
    let service = service(SequenceIds::default());
    let created = service
        .create_job(key(), request("west"))
        .unwrap_or_else(|error| panic!("create must succeed: {error}"));
    let CreateJobOutcome::Created(job) = created else {
        panic!("first create must create a job");
    };

    let failed = service
        .mark_failed(&job.id)
        .unwrap_or_else(|error| panic!("accepted job must fail: {error}"));
    assert_eq!(failed.state, JobState::Failed);
    assert_eq!(service.get_job(&job.id).ok().flatten(), Some(failed));
    assert!(matches!(
        service.mark_failed(&job.id),
        Err(DispatchServiceError::JobNotAccepted {
            state: JobState::Failed,
            ..
        })
    ));
}

#[test]
fn an_unhealthy_queue_refuses_new_work_with_public_queue_evidence() {
    let service = DispatchService::with_generator(
        SequenceIds::default(),
        admission(HealthSeverity::Unhealthy, 16),
    );

    let refused = service
        .create_job(key(), request("west"))
        .unwrap_or_else(|error| panic!("refusal is an outcome: {error}"));
    assert!(matches!(
        refused,
        CreateJobOutcome::Refused(AdmissionRefusal::QueueUnavailable { .. })
    ));
}

#[test]
fn one_service_that_stopped_admitting_work_still_replays_an_existing_key() {
    // A queue observation is fixed when the service is built, so the accepted
    // backlog is the one admission input that can worsen while a service runs.
    // Capacity one makes this instance admit the first job and refuse after it,
    // which is what an outage arriving after a key exists looks like here.
    let service = DispatchService::with_generator(
        SequenceIds::default(),
        admission(HealthSeverity::Degraded, 1),
    );
    let created = service
        .create_job(key(), request("west"))
        .unwrap_or_else(|error| panic!("first create must succeed: {error}"));
    let CreateJobOutcome::Created(job) = created else {
        panic!("first create must create a job");
    };
    let new_key = IdempotencyKey::new("create-job-test-2")
        .unwrap_or_else(|error| panic!("test key must be valid: {error}"));

    let refused = service
        .create_job(new_key, request("east"))
        .unwrap_or_else(|error| panic!("refusal is an outcome: {error}"));
    assert!(matches!(
        refused,
        CreateJobOutcome::Refused(AdmissionRefusal::CapacityExhausted { .. })
    ));
    // The same instance is refusing new work, so a replay can only succeed
    // because create_job resolves an existing key before consulting admission.
    assert!(matches!(
        service.create_job(key(), request("west")),
        Ok(CreateJobOutcome::Replayed(ref value)) if value == &job
    ));
}

#[test]
fn failing_an_unknown_job_is_an_explicit_service_error() {
    let service = service(SequenceIds::default());
    let job_id = JobId::new("job_01K00000000000000000000099")
        .unwrap_or_else(|error| panic!("test job ID must be valid: {error}"));

    assert!(matches!(
        service.mark_failed(&job_id),
        Err(DispatchServiceError::JobNotFound { job_id: missing }) if missing == job_id
    ));
}
