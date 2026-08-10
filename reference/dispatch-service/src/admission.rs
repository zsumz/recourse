//! Framework-neutral decision to accept, or transiently refuse, new work.

use std::time::Duration;

use dispatch_diagnostics::QueueUnavailableEvidence;
use recourse::{health::HealthSeverity, http::RetryAfter};

use crate::QueueObservation;

/// Accepted-backlog size the reference service admits before refusing.
pub const DEFAULT_ADMISSION_CAPACITY: usize = 1024;

/// Minimum delay Dispatch publishes while a transient condition clears.
pub const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(30);

/// Policy deciding whether Dispatch can accept another background job.
///
/// Both refusals are transient dependency conditions rather than caller
/// mistakes, so the policy also owns the minimum delay callers should wait.
/// That delay is application knowledge, not transport knowledge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobAdmission {
    queue: QueueObservation,
    capacity: usize,
    retry_after: Duration,
}

impl JobAdmission {
    /// Admits work while the queue is usable and the backlog fits.
    pub const fn new(queue: QueueObservation, capacity: usize, retry_after: Duration) -> Self {
        Self {
            queue,
            capacity,
            retry_after,
        }
    }

    /// Uses the reference capacity and minimum retry delay.
    pub const fn with_defaults(queue: QueueObservation) -> Self {
        Self::new(queue, DEFAULT_ADMISSION_CAPACITY, DEFAULT_RETRY_DELAY)
    }

    /// Current queue condition this policy admits or refuses against.
    pub const fn queue(&self) -> &QueueObservation {
        &self.queue
    }

    /// Largest accepted backlog the service will hold.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) fn refusal(&self, accepted: usize) -> Option<AdmissionRefusal> {
        let retry_after = RetryAfter::after(self.retry_after);
        // A degraded queue is impaired but still usable, so only an unhealthy
        // queue withdraws the capability the caller is asking for.
        if matches!(self.queue.severity(), HealthSeverity::Unhealthy) {
            return Some(AdmissionRefusal::QueueUnavailable {
                evidence: self.queue.evidence(),
                retry_after,
            });
        }
        (accepted >= self.capacity).then_some(AdmissionRefusal::CapacityExhausted { retry_after })
    }
}

/// Transient reason Dispatch cannot accept another job right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRefusal {
    /// Workers cannot currently reach the durable job queue.
    QueueUnavailable {
        /// Public facts describing the observed outage.
        evidence: QueueUnavailableEvidence,
        /// Minimum delay published to the caller.
        retry_after: RetryAfter,
    },
    /// The accepted backlog is already at its configured capacity.
    CapacityExhausted {
        /// Minimum delay published to the caller.
        retry_after: RetryAfter,
    },
}
