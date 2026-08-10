//! Public worker health publication and exact finding fixture.

use std::time::Duration;

use dispatch_diagnostics::catalog;
use dispatch_service::{DispatchService, JobAdmission, QueueObservation};
use dispatch_worker::DispatchWorker;
use recourse::health::{HealthFindingId, HealthSeverity, ObservationTime};

fn degraded_service() -> DispatchService {
    let finding_id = HealthFindingId::try_new("finding_queue-unavailable")
        .unwrap_or_else(|error| panic!("fixture finding ID must be valid: {error}"));
    let observed_at = ObservationTime::parse("2026-08-10T14:31:00Z")
        .unwrap_or_else(|error| panic!("fixture observation time must be valid: {error}"));
    DispatchService::new(JobAdmission::new(
        QueueObservation::new(finding_id, HealthSeverity::Degraded, observed_at, 3),
        16,
        Duration::from_secs(30),
    ))
}

#[test]
fn worker_publishes_one_exact_typed_queue_finding() {
    let worker = DispatchWorker::new();
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let service = degraded_service();

    let published = worker
        .publish_queue_health(&catalog, &service)
        .unwrap_or_else(|error| panic!("queue finding must publish: {error}"));
    let fixture = include_bytes!("../../../conformance/wire/dispatch-health-finding.json");
    let expected = serde_json::from_slice::<serde_json::Value>(fixture)
        .unwrap_or_else(|error| panic!("fixture must be canonical JSON: {error}"));

    assert_eq!(published.body(), &expected);
    assert_eq!(
        worker.published_queue_health().ok().flatten(),
        Some(published)
    );
}

/// The published body is a `serde_json::Value`, which compares by member name.
/// This pins the canonical byte order of the same finding the worker publishes.
#[test]
fn the_published_finding_encodes_to_the_exact_fixture_bytes() {
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let finding = degraded_service()
        .try_queue_finding(&catalog)
        .unwrap_or_else(|error| panic!("queue finding must build: {error}"));

    let encoded = finding
        .try_encode()
        .unwrap_or_else(|error| panic!("finding must encode: {error}"));
    let fixture = include_bytes!("../../../conformance/wire/dispatch-health-finding.json");
    assert_eq!(encoded, fixture.strip_suffix(b"\n").unwrap_or(fixture));
}
