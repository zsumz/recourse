//! Closed severity vocabulary for current unhealthy service state.

use serde::{Deserialize, Serialize};

/// Severity of one current health finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthSeverity {
    /// Service remains usable with a material impairment.
    Degraded,
    /// Service cannot provide the affected capability.
    Unhealthy,
}
