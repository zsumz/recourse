//! Transient refusal thresholds and their published retry delays.

use std::time::Duration;

use recourse::{
    health::{HealthFindingId, HealthSeverity, ObservationTime},
    http::RetryAfter,
};

use super::{AdmissionRefusal, JobAdmission, QueueObservation};

fn observation(severity: HealthSeverity) -> QueueObservation {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("fixture finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture observation time must be valid: {error}"));
    QueueObservation::new(finding_id, severity, observed_at, 3)
}

#[test]
fn a_degraded_queue_still_admits_work_within_capacity() {
    let admission = JobAdmission::new(
        observation(HealthSeverity::Degraded),
        2,
        Duration::from_secs(30),
    );

    assert_eq!(admission.refusal(1), None);
}

#[test]
fn an_unhealthy_queue_refuses_with_typed_evidence_and_a_delay() {
    let admission = JobAdmission::with_defaults(observation(HealthSeverity::Unhealthy));

    let refusal = admission.refusal(0);
    assert!(matches!(
        refusal,
        Some(AdmissionRefusal::QueueUnavailable { evidence, retry_after })
            if evidence.consecutive_failures == 3
                && retry_after == RetryAfter::after(Duration::from_secs(30))
    ));
}

#[test]
fn a_full_accepted_backlog_refuses_without_public_evidence() {
    let admission = JobAdmission::new(
        observation(HealthSeverity::Degraded),
        1,
        Duration::from_secs(45),
    );

    assert!(matches!(
        admission.refusal(1),
        Some(AdmissionRefusal::CapacityExhausted { retry_after })
            if retry_after == RetryAfter::after(Duration::from_secs(45))
    ));
}
