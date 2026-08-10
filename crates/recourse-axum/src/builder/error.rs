//! Actionable layer configuration failures.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::StatusCode;
use recourse::http::{
    CorrelationIdError, ProblemBuildError, ProblemEncodeError, ProblemOccurrenceError,
};

/// Invalid or incomplete Axum adapter configuration.
#[derive(Debug)]
pub enum LayerBuildError {
    /// No internal fallback diagnostic was selected.
    MissingInternal,
    /// Neither a fault reporter nor a deliberate discard was selected.
    MissingFaultReporter,
    /// A fault reporter and a deliberate discard were both selected.
    ContradictoryFaultReporting,
    /// Selected diagnostic was absent or violated catalog policy.
    InternalProblem(ProblemBuildError),
    /// Selected diagnostic could not produce canonical JSON.
    InternalEncoding(ProblemEncodeError),
    /// Selected diagnostic did not use a server-error status.
    InternalStatus {
        /// Rejected governed status.
        status: StatusCode,
    },
    /// Adapter's fixed validation correlation value was rejected.
    ValidationRequestId(CorrelationIdError),
    /// Adapter's fixed validation occurrence value was rejected.
    ValidationOccurrence(ProblemOccurrenceError),
}

impl Display for LayerBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingInternal => {
                formatter.write_str("internal fallback diagnostic is required")
            }
            Self::MissingFaultReporter => formatter.write_str(
                "call fault_reporter(..) to receive private reports or discard_faults() to drop them",
            ),
            Self::ContradictoryFaultReporting => formatter.write_str(
                "fault_reporter(..) and discard_faults() state opposite choices; call exactly one",
            ),
            Self::InternalProblem(error) => {
                write!(formatter, "invalid internal diagnostic: {error}")
            }
            Self::InternalEncoding(error) => {
                write!(formatter, "encode internal diagnostic: {error}")
            }
            Self::InternalStatus { status } => {
                write!(
                    formatter,
                    "internal diagnostic status {status} is not a 5xx status"
                )
            }
            Self::ValidationRequestId(error) => {
                write!(formatter, "invalid adapter validation request ID: {error}")
            }
            Self::ValidationOccurrence(error) => {
                write!(formatter, "invalid adapter validation occurrence: {error}")
            }
        }
    }
}

impl Error for LayerBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InternalProblem(error) => Some(error),
            Self::InternalEncoding(error) => Some(error),
            Self::ValidationRequestId(error) => Some(error),
            Self::ValidationOccurrence(error) => Some(error),
            Self::MissingInternal
            | Self::MissingFaultReporter
            | Self::ContradictoryFaultReporting
            | Self::InternalStatus { .. } => None,
        }
    }
}
