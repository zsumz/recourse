//! Durable record returned by the reference worker.

use dispatch_diagnostics::DispatchImpact;
use dispatch_model::Job;
use recourse::operation::OperationDiagnosticId;
use serde_json::Value;

/// Canonically encoded operation diagnostic and its failed job state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedDispatchFailure {
    diagnostic_id: OperationDiagnosticId,
    job: Job,
    attempt: u32,
    impact: DispatchImpact,
    body: Value,
}

impl RecordedDispatchFailure {
    pub(crate) const fn new(
        diagnostic_id: OperationDiagnosticId,
        job: Job,
        attempt: u32,
        impact: DispatchImpact,
        body: Value,
    ) -> Self {
        Self {
            diagnostic_id,
            job,
            attempt,
            impact,
            body,
        }
    }

    /// Stable identity of this durable diagnostic occurrence.
    pub const fn diagnostic_id(&self) -> &OperationDiagnosticId {
        &self.diagnostic_id
    }

    /// Job after its accepted-to-failed transition.
    pub const fn job(&self) -> &Job {
        &self.job
    }

    /// Canonical Recourse operation-diagnostic document stored by the worker.
    pub const fn body(&self) -> &Value {
        &self.body
    }

    pub(crate) fn matches(&self, attempt: u32, impact: &DispatchImpact) -> bool {
        self.attempt == attempt && &self.impact == impact
    }
}

/// Result of an idempotent durable-failure write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordFailureOutcome {
    /// The worker recorded a new failure and transitioned the job.
    Recorded(RecordedDispatchFailure),
    /// The same attempt and impact were already recorded.
    Replayed(RecordedDispatchFailure),
}

impl RecordFailureOutcome {
    /// Borrows the durable record regardless of whether it was newly written.
    pub const fn record(&self) -> &RecordedDispatchFailure {
        match self {
            Self::Recorded(record) | Self::Replayed(record) => record,
        }
    }
}
