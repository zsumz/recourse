//! Explicit failures while recording durable Dispatch outcomes.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use dispatch_service::DispatchFault;
use recourse::{
    health::HealthEncodeError,
    operation::{OperationDiagnosticId, OperationEncodeError},
};

/// Failure to build, transition, or persist one durable diagnostic.
#[derive(Debug)]
pub enum DispatchWorkerError {
    /// The framework-neutral service could not produce a governed value.
    Diagnostic(DispatchFault),
    /// Typed public data could not be encoded canonically.
    Encode(OperationEncodeError),
    /// Typed health evidence could not be encoded canonically.
    HealthEncode(HealthEncodeError),
    /// A replay reused an occurrence identity with different public impact.
    ConflictingReplay {
        /// Existing stable occurrence identity.
        diagnostic_id: OperationDiagnosticId,
    },
    /// A previous panic poisoned the in-memory reference record store.
    RecordStorePoisoned,
    /// A previous panic poisoned the current health publication.
    HealthStorePoisoned,
}

impl Display for DispatchWorkerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Diagnostic(fault) => write!(formatter, "build governed diagnostic: {fault}"),
            Self::Encode(error) => write!(formatter, "encode operation diagnostic: {error}"),
            Self::HealthEncode(error) => write!(formatter, "encode health finding: {error}"),
            Self::ConflictingReplay { diagnostic_id } => {
                write!(
                    formatter,
                    "diagnostic {diagnostic_id} has conflicting impact"
                )
            }
            Self::RecordStorePoisoned => formatter.write_str("failure record store is poisoned"),
            Self::HealthStorePoisoned => formatter.write_str("health publication is poisoned"),
        }
    }
}

impl Error for DispatchWorkerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Diagnostic(fault) => Some(fault),
            Self::Encode(error) => Some(error),
            Self::HealthEncode(error) => Some(error),
            Self::ConflictingReplay { .. }
            | Self::RecordStorePoisoned
            | Self::HealthStorePoisoned => None,
        }
    }
}
