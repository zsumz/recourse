//! Construction and strict encoding failures for health findings.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::{catalog::Code, wire::WireLimitError};

/// Failure to construct a governed health finding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HealthBuildError {
    /// Marker was not registered on the health-finding surface.
    DiagnosticNotRegistered {
        /// Rust marker name for the application author.
        diagnostic: &'static str,
    },
    /// Validated catalog unexpectedly has no compiled evidence validator.
    ValidatorMissing {
        /// Diagnostic whose runtime contract is incomplete.
        code: Code,
    },
}

impl Display for HealthBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiagnosticNotRegistered { diagnostic } => {
                write!(formatter, "health finding {diagnostic} is not registered")
            }
            Self::ValidatorMissing { code } => {
                write!(
                    formatter,
                    "health finding {code} has no compiled evidence validator"
                )
            }
        }
    }
}

impl Error for HealthBuildError {}

/// Failure to produce the strict health-finding wire profile.
#[derive(Debug)]
#[non_exhaustive]
pub enum HealthEncodeError {
    /// Evidence's custom serializer failed.
    EvidenceSerialization(serde_json::Error),
    /// Evidence serialized to a non-object despite its public schema.
    EvidenceNotObject,
    /// Runtime evidence disagrees with the accepted schema.
    EvidenceSchemaMismatch {
        /// JSON path of the first mismatch.
        path: String,
        /// Validator explanation.
        reason: String,
    },
    /// Emitted JSON would exceed the shared protocol profile.
    WireLimit(WireLimitError),
    /// Final canonical body serialization failed.
    BodySerialization(serde_json::Error),
}

impl Display for HealthEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceSerialization(error) => write!(formatter, "serialize evidence: {error}"),
            Self::EvidenceNotObject => formatter.write_str("evidence must serialize as an object"),
            Self::EvidenceSchemaMismatch { path, reason } => {
                write!(
                    formatter,
                    "evidence violates its schema at {path}: {reason}"
                )
            }
            Self::WireLimit(error) => Display::fmt(error, formatter),
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
            Self::WireLimit(error) => Some(error),
            Self::EvidenceNotObject | Self::EvidenceSchemaMismatch { .. } => None,
        }
    }
}
