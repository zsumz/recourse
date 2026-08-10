//! Current durable-queue condition and its governed health finding.

use dispatch_diagnostics::{DispatchCatalog, QueueUnavailable, QueueUnavailableEvidence};
use recourse::{
    catalog::Catalog,
    health::{HealthFinding, HealthFindingId, HealthSeverity, ObservationTime},
};

use crate::{DispatchFault, DispatchService, JobIdGenerator};

/// Typed application observation of the durable job queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueObservation {
    finding_id: HealthFindingId,
    severity: HealthSeverity,
    observed_at: ObservationTime,
    consecutive_failures: u32,
}

impl QueueObservation {
    /// Describes one current degraded or unhealthy queue condition.
    pub const fn new(
        finding_id: HealthFindingId,
        severity: HealthSeverity,
        observed_at: ObservationTime,
        consecutive_failures: u32,
    ) -> Self {
        Self {
            finding_id,
            severity,
            observed_at,
            consecutive_failures,
        }
    }

    /// Stable identity for the currently observed condition.
    pub const fn finding_id(&self) -> &HealthFindingId {
        &self.finding_id
    }

    /// Current governed severity.
    pub const fn severity(&self) -> HealthSeverity {
        self.severity
    }

    /// Canonical observation timestamp.
    pub const fn observed_at(&self) -> &ObservationTime {
        &self.observed_at
    }

    /// Number of consecutive failed connectivity probes.
    pub const fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Caller-visible facts shared by the finding and the request refusal.
    pub const fn evidence(&self) -> QueueUnavailableEvidence {
        QueueUnavailableEvidence {
            consecutive_failures: self.consecutive_failures,
        }
    }
}

impl<G: JobIdGenerator> DispatchService<G> {
    /// Builds the governed finding for the service's current queue condition.
    ///
    /// The finding is a resource value rather than a failed-request envelope,
    /// so the worker publishes it without any HTTP status attached.
    pub fn try_queue_finding(
        &self,
        catalog: &Catalog<DispatchCatalog>,
    ) -> Result<HealthFinding<QueueUnavailableEvidence>, DispatchFault> {
        let queue = self.admission().queue();
        catalog
            .try_health::<QueueUnavailable>(
                queue.finding_id().clone(),
                queue.severity(),
                queue.observed_at().clone(),
                queue.evidence(),
            )
            .map_err(|error| {
                DispatchFault::new(error, "build_queue_finding")
                    .with("finding_id", queue.finding_id().as_str())
            })
    }
}
