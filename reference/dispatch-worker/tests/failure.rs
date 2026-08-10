//! Public worker workflow and exact durable-diagnostic fixture.

use std::time::Duration;

use dispatch_diagnostics::{DispatchImpact, catalog};
use dispatch_model::{
    CreateJobRequest, Destination, IdempotencyKey, Job, JobId, JobIdError, JobState,
};
use dispatch_service::{
    CreateJobOutcome, DispatchFailure, DispatchService, JobAdmission, JobIdGenerator,
    QueueObservation,
};
use dispatch_worker::{DispatchWorker, DispatchWorkerError, RecordFailureOutcome};
use recourse::health::{HealthFindingId, HealthSeverity, ObservationTime};

#[derive(Debug, Clone, Copy)]
struct FixedJobId;

impl JobIdGenerator for FixedJobId {
    fn generate(&self) -> Result<JobId, JobIdError> {
        JobId::new("job_01K00000000000000000000000")
    }
}

fn admission() -> JobAdmission {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("fixture finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture observation time must be valid: {error}"));
    JobAdmission::new(
        QueueObservation::new(finding_id, HealthSeverity::Degraded, observed_at, 3),
        16,
        Duration::from_secs(30),
    )
}

fn accepted_job() -> (DispatchService<FixedJobId>, Job) {
    let service = DispatchService::with_generator(FixedJobId, admission());
    let key = IdempotencyKey::new("worker-fixture")
        .unwrap_or_else(|error| panic!("fixture key must be valid: {error}"));
    let destination = Destination::new("west")
        .unwrap_or_else(|error| panic!("fixture destination must be valid: {error}"));
    let outcome = service
        .create_job(key, CreateJobRequest { destination })
        .unwrap_or_else(|error| panic!("fixture job must be accepted: {error}"));
    let CreateJobOutcome::Created(job) = outcome else {
        panic!("first fixture request must create a job");
    };
    (service, job)
}

fn failure(job: &Job, created_artifacts: u32) -> DispatchFailure {
    DispatchFailure::new(
        job.id.clone(),
        3,
        DispatchImpact {
            destination_changed: false,
            created_artifacts,
        },
    )
}

#[test]
fn accepted_job_records_one_exact_typed_durable_failure() {
    let (service, job) = accepted_job();
    let worker = DispatchWorker::new();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));

    let outcome = worker
        .record_failure(&catalog, &service, &failure(&job, 2))
        .unwrap_or_else(|error| panic!("accepted failure must record: {error}"));
    let RecordFailureOutcome::Recorded(record) = outcome else {
        panic!("first failure write must be new");
    };

    assert_eq!(record.job().state, JobState::Failed);
    let fixture = include_bytes!("../../../conformance/wire/dispatch-operation.json");
    let expected = serde_json::from_slice::<serde_json::Value>(fixture)
        .unwrap_or_else(|error| panic!("fixture must be canonical JSON: {error}"));
    assert_eq!(record.body(), &expected);
    assert_eq!(
        worker
            .recorded_failure(record.diagnostic_id())
            .ok()
            .flatten(),
        Some(record)
    );
}

/// The recorded body is a `serde_json::Value`, which compares by member name.
/// This pins the canonical byte order of the same diagnostic the worker records.
#[test]
fn the_recorded_diagnostic_encodes_to_the_exact_fixture_bytes() {
    let (service, job) = accepted_job();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let failed = service
        .try_fail_job(&catalog, &failure(&job, 2))
        .unwrap_or_else(|error| panic!("accepted job must fail: {error}"));

    let encoded = failed
        .diagnostic()
        .try_encode()
        .unwrap_or_else(|error| panic!("diagnostic must encode: {error}"));
    let fixture = include_bytes!("../../../conformance/wire/dispatch-operation.json");
    assert_eq!(encoded, fixture.strip_suffix(b"\n").unwrap_or(fixture));
}

#[test]
fn exact_replay_is_idempotent_but_changed_impact_conflicts() {
    let (service, job) = accepted_job();
    let worker = DispatchWorker::new();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    assert!(
        worker
            .record_failure(&catalog, &service, &failure(&job, 2))
            .is_ok()
    );

    assert!(matches!(
        worker.record_failure(&catalog, &service, &failure(&job, 2)),
        Ok(RecordFailureOutcome::Replayed(_))
    ));
    assert!(matches!(
        worker.record_failure(&catalog, &service, &failure(&job, 1)),
        Err(DispatchWorkerError::ConflictingReplay { .. })
    ));
}

#[test]
fn an_unknown_job_surfaces_the_services_private_report() {
    let (service, _) = accepted_job();
    let worker = DispatchWorker::new();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let unknown = JobId::new("job_01K00000000000000000000099")
        .unwrap_or_else(|error| panic!("fixture job ID must be valid: {error}"));

    let error = worker
        .record_failure(
            &catalog,
            &service,
            &DispatchFailure::new(
                unknown,
                1,
                DispatchImpact {
                    destination_changed: false,
                    created_artifacts: 0,
                },
            ),
        )
        .err()
        .unwrap_or_else(|| panic!("an unknown job cannot be recorded"));

    assert!(matches!(error, DispatchWorkerError::Diagnostic(_)));
    assert!(error.to_string().contains("[operation=fail_job]"));
}
