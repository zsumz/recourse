//! Explicit construction and encoding failure taxonomy.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::catalog::Code;

use super::super::PolicyError;

/// Strict Problem construction failure.
#[derive(Debug)]
pub enum ProblemBuildError {
    /// Exact diagnostic marker type was not registered in this catalog.
    DiagnosticNotRegistered {
        /// Rust marker type name for private programmer diagnostics.
        diagnostic: &'static str,
    },
    /// Policy declared an invalid HTTP status value.
    InvalidPolicyStatus {
        /// Rejected numeric status.
        status: u16,
    },
    /// Runtime policy no longer agrees with the validated catalog.
    CatalogPolicyMismatch {
        /// Permanent diagnostic identity.
        code: Code,
        /// Status recorded during catalog validation.
        catalog_status: u16,
        /// Status supplied by the diagnostic policy.
        policy_status: u16,
    },
    /// Typed runtime policy input was rejected.
    Policy(PolicyError),
    /// Policy failed to construct one of its declared required headers.
    MissingPolicyHeader {
        /// Missing canonical header name.
        name: &'static str,
    },
}

impl Display for ProblemBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DiagnosticNotRegistered { diagnostic } => {
                write!(formatter, "diagnostic {diagnostic} is not registered")
            }
            Self::InvalidPolicyStatus { status } => write!(formatter, "invalid status {status}"),
            Self::CatalogPolicyMismatch {
                code,
                catalog_status,
                policy_status,
            } => write!(
                formatter,
                "{code} catalog status {catalog_status} disagrees with policy {policy_status}"
            ),
            Self::Policy(error) => write!(formatter, "invalid policy input: {error}"),
            Self::MissingPolicyHeader { name } => {
                write!(formatter, "policy did not construct required header {name}")
            }
        }
    }
}

impl Error for ProblemBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            _ => None,
        }
    }
}

/// Canonical Problem encoding failure.
#[derive(Debug)]
pub enum ProblemEncodeError {
    /// Reviewed evidence serializer returned an error.
    EvidenceSerialization(serde_json::Error),
    /// Runtime evidence representation is not a JSON object.
    EvidenceNotObject,
    /// Canonical top-level body serialization failed.
    BodySerialization(serde_json::Error),
}

impl Display for ProblemEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EvidenceSerialization(error) => write!(formatter, "serialize evidence: {error}"),
            Self::EvidenceNotObject => formatter.write_str("public evidence must encode as object"),
            Self::BodySerialization(error) => write!(formatter, "serialize Problem body: {error}"),
        }
    }
}

impl Error for ProblemEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EvidenceSerialization(error) | Self::BodySerialization(error) => Some(error),
            Self::EvidenceNotObject => None,
        }
    }
}
