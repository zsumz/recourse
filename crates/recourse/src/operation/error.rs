//! Construction and strict encoding failures for durable diagnostics.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Failure to construct a governed operation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationBuildError {
    /// Marker was not registered on the durable-operation surface.
    DiagnosticNotRegistered {
        /// Rust marker name for the application author.
        diagnostic: &'static str,
    },
}

impl Display for OperationBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiagnosticNotRegistered { diagnostic } => {
                write!(
                    formatter,
                    "operation diagnostic {diagnostic} is not registered"
                )
            }
        }
    }
}

impl Error for OperationBuildError {}

/// Failure to produce the strict durable-diagnostic wire profile.
#[derive(Debug)]
pub enum OperationEncodeError {
    /// Evidence's custom serializer failed.
    EvidenceSerialization(serde_json::Error),
    /// Evidence serialized to a non-object despite its public schema.
    EvidenceNotObject,
    /// Impact's custom serializer failed.
    ImpactSerialization(serde_json::Error),
    /// Impact serialized to a non-object despite its public schema.
    ImpactNotObject,
    /// Final canonical body serialization failed.
    BodySerialization(serde_json::Error),
}

impl Display for OperationEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceSerialization(error) => write!(formatter, "serialize evidence: {error}"),
            Self::EvidenceNotObject => formatter.write_str("evidence must serialize as an object"),
            Self::ImpactSerialization(error) => write!(formatter, "serialize impact: {error}"),
            Self::ImpactNotObject => formatter.write_str("impact must serialize as an object"),
            Self::BodySerialization(error) => {
                write!(formatter, "serialize operation diagnostic body: {error}")
            }
        }
    }
}

impl Error for OperationEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EvidenceSerialization(error)
            | Self::ImpactSerialization(error)
            | Self::BodySerialization(error) => Some(error),
            Self::EvidenceNotObject | Self::ImpactNotObject => None,
        }
    }
}
