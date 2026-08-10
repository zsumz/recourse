//! Construction and strict encoding failures for health findings.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Failure to construct a governed health finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthBuildError {
    /// Marker was not registered on the health-finding surface.
    DiagnosticNotRegistered {
        /// Rust marker name for the application author.
        diagnostic: &'static str,
    },
}

impl Display for HealthBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiagnosticNotRegistered { diagnostic } => {
                write!(formatter, "health finding {diagnostic} is not registered")
            }
        }
    }
}

impl Error for HealthBuildError {}

/// Failure to produce the strict health-finding wire profile.
#[derive(Debug)]
pub enum HealthEncodeError {
    /// Evidence's custom serializer failed.
    EvidenceSerialization(serde_json::Error),
    /// Evidence serialized to a non-object despite its public schema.
    EvidenceNotObject,
    /// Final canonical body serialization failed.
    BodySerialization(serde_json::Error),
}

impl Display for HealthEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceSerialization(error) => write!(formatter, "serialize evidence: {error}"),
            Self::EvidenceNotObject => formatter.write_str("evidence must serialize as an object"),
            Self::BodySerialization(error) => {
                write!(formatter, "serialize health finding body: {error}")
            }
        }
    }
}

impl Error for HealthEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EvidenceSerialization(error) | Self::BodySerialization(error) => Some(error),
            Self::EvidenceNotObject => None,
        }
    }
}
