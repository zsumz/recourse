//! Service-owned durable diagnostics, health findings, and private reports.

use std::time::Duration;

use dispatch_diagnostics::{DispatchImpact, catalog};
use dispatch_model::{CreateJobRequest, Destination, IdempotencyKey, Job, JobId, JobState};
use dispatch_service::{
    CreateJobOutcome, DispatchFailure, DispatchService, JobAdmission, QueueObservation, UlidJobIds,
};
use recourse::health::{HealthFindingId, HealthSeverity, ObservationTime};

fn admission(severity: HealthSeverity) -> JobAdmission {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("fixture finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture observation time must be valid: {error}"));
    JobAdmission::new(
        QueueObservation::new(finding_id, severity, observed_at, 3),
        16,
        Duration::from_secs(30),
    )
}

fn accepted_job() -> (DispatchService<UlidJobIds>, Job) {
    let service = DispatchService::new(admission(HealthSeverity::Degraded));
    let key = IdempotencyKey::new("service-diagnostics")
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

fn failure(job_id: JobId) -> DispatchFailure {
    DispatchFailure::new(
        job_id,
        3,
        DispatchImpact {
            destination_changed: false,
            created_artifacts: 2,
        },
    )
}

#[test]
fn failing_a_job_transitions_it_and_builds_its_durable_diagnostic() {
    let (service, job) = accepted_job();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));

    let failed = service
        .try_fail_job(&catalog, &failure(job.id.clone()))
        .unwrap_or_else(|error| panic!("accepted job must fail: {error}"));

    assert_eq!(failed.job().state, JobState::Failed);
    let diagnostic = failed.diagnostic();
    assert_eq!(diagnostic.code().to_string(), "DSP-1009");
    assert_eq!(diagnostic.evidence().attempt, 3);
    assert_eq!(diagnostic.impact().created_artifacts, 2);
    assert!(
        diagnostic
            .id()
            .as_str()
            .ends_with(&format!("{}-3", &job.id.as_str()["job_".len()..]))
    );
}

#[test]
fn a_second_failure_report_is_an_operator_only_private_report() {
    let (service, job) = accepted_job();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    assert!(
        service
            .try_fail_job(&catalog, &failure(job.id.clone()))
            .is_ok()
    );

    let fault = service
        .try_fail_job(&catalog, &failure(job.id.clone()))
        .err()
        .unwrap_or_else(|| panic!("a failed job cannot fail twice"));

    let rendered = fault.report().to_string();
    assert!(rendered.contains("cannot fail from state Failed"));
    assert!(rendered.contains("[operation=fail_job]"));
    assert!(rendered.contains(&format!("[job_id={}]", job.id)));
    assert_eq!(fault.into_report().contexts().len(), 2);
}

#[test]
fn the_service_builds_the_finding_for_its_own_queue_condition() {
    let service = DispatchService::new(admission(HealthSeverity::Unhealthy));
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));

    let finding = service
        .try_queue_finding(&catalog)
        .unwrap_or_else(|error| panic!("registered finding must build: {error}"));

    assert_eq!(finding.code().to_string(), "DSP-1010");
    assert_eq!(finding.severity(), HealthSeverity::Unhealthy);
    assert_eq!(finding.evidence().consecutive_failures, 3);
    assert_eq!(finding.observed_at().as_str(), "2026-08-10T14:31:00Z");
    assert_eq!(finding.id().as_str(), "finding_queue-unavailable");
}
