//! Current service-state finding declarations and envelopes.

mod declaration;
mod error;
mod finding;
mod id;
mod observation;
mod severity;

pub use declaration::HealthFindingType;
pub use error::{HealthBuildError, HealthEncodeError};
pub use finding::HealthFinding;
pub use id::{HealthFindingId, HealthFindingIdError, MAX_HEALTH_FINDING_ID_BYTES};
pub use observation::{ObservationTime, ObservationTimeError};
pub use severity::HealthSeverity;

#[cfg(test)]
mod finding_test;
#[cfg(test)]
mod identity_test;
