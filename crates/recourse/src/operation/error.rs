//! Construction and strict encoding failures for durable diagnostics.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::{catalog::Code, wire::WireLimitError};

/// Failure to construct a governed operation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationBuildError {
    /// Marker was not registered on the durable-operation surface.
    DiagnosticNotRegistered {
        /// Rust marker name for the application author.
        diagnostic: &'static str,
    },
    /// Validated catalog unexpectedly has no compiled value validators.
    ValidatorsMissing {
        /// Diagnostic whose runtime contract is incomplete.
        code: Code,
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
            Self::ValidatorsMissing { code } => {
                write!(
                    formatter,
                    "operation diagnostic {code} has no compiled validators"
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
    /// Runtime evidence disagrees with the accepted schema.
    EvidenceSchemaMismatch {
        /// JSON path of the first mismatch.
        path: String,
        /// Validator explanation.
        reason: String,
    },
    /// Impact's custom serializer failed.
    ImpactSerialization(serde_json::Error),
    /// Impact serialized to a non-object despite its public schema.
    ImpactNotObject,
    /// Runtime impact disagrees with the accepted schema.
    ImpactSchemaMismatch {
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

impl Display for OperationEncodeError {
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
            Self::ImpactSerialization(error) => write!(formatter, "serialize impact: {error}"),
            Self::ImpactNotObject => formatter.write_str("impact must serialize as an object"),
            Self::ImpactSchemaMismatch { path, reason } => {
                write!(formatter, "impact violates its schema at {path}: {reason}")
            }
            Self::WireLimit(error) => Display::fmt(error, formatter),
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
            Self::WireLimit(error) => Some(error),
            Self::EvidenceNotObject
            | Self::EvidenceSchemaMismatch { .. }
            | Self::ImpactNotObject
            | Self::ImpactSchemaMismatch { .. } => None,
        }
    }
}
