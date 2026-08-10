//! Dispatch-owned caller-visible evidence objects.

use dispatch_model::JobId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use recourse::diagnostic::PublicEvidence;

/// Public identity needed to correct a job lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct JobNotFoundEvidence {
    /// Job identifier that was not found.
    pub job_id: JobId,
}

impl PublicEvidence for JobNotFoundEvidence {}

/// Public identity of the request originally bound to an idempotency key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IdempotencyConflictEvidence {
    /// Existing job created by the first request using the key.
    pub original_job_id: JobId,
}

impl PublicEvidence for IdempotencyConflictEvidence {}

/// Public attempt facts recorded when accepted work fails durably.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DispatchFailedEvidence {
    /// Accepted job whose background dispatch failed.
    pub job_id: JobId,
    /// Dispatch attempt that failed.
    pub attempt: u32,
}

impl PublicEvidence for DispatchFailedEvidence {}

/// Public consequences of a failed dispatch attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DispatchImpact {
    /// Whether the configured destination was mutated.
    pub destination_changed: bool,
    /// Number of durable artifacts created before failure.
    pub created_artifacts: u32,
}

impl PublicEvidence for DispatchImpact {}

/// Public facts describing repeated queue connectivity failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct QueueUnavailableEvidence {
    /// Consecutive failed connectivity probes.
    pub consecutive_failures: u32,
}

impl PublicEvidence for QueueUnavailableEvidence {}
