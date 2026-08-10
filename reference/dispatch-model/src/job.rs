//! Public request, job, and lifecycle representations.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Destination, JobId};

/// Validated inputs used to create one background job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateJobRequest {
    /// Public destination for the background dispatch.
    pub destination: Destination,
}

/// Current durable lifecycle state of a Dispatch job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Job has been accepted for asynchronous processing.
    Accepted,
    /// Worker completed the job successfully.
    Completed,
    /// Worker recorded a durable operation diagnostic.
    Failed,
}

/// Public representation of one accepted background job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Job {
    /// Permanent job identity.
    pub id: JobId,
    /// Validated destination supplied at creation.
    pub destination: Destination,
    /// Current lifecycle state.
    pub state: JobState,
}
