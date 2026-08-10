//! Canonically encoded queue finding currently published by the worker.

use dispatch_service::QueueObservation;
use serde_json::Value;

/// Current queue finding a worker has published for the health resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedQueueHealth {
    observation: QueueObservation,
    body: Value,
}

impl PublishedQueueHealth {
    pub(crate) const fn new(observation: QueueObservation, body: Value) -> Self {
        Self { observation, body }
    }

    /// Application observation represented by this finding.
    pub const fn observation(&self) -> &QueueObservation {
        &self.observation
    }

    /// Canonical Recourse health-finding document.
    pub const fn body(&self) -> &Value {
        &self.body
    }
}
