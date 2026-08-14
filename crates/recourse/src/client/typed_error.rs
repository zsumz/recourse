//! Failures that prevent a received Problem from becoming a typed view.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::StatusCode;

use crate::catalog::CodeParseError;

/// Typed known-code verification or evidence-decoding failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum TypedProblemError {
    /// Diagnostic's catalog declaration cannot produce a canonical code.
    InvalidDeclaration(CodeParseError),
    /// Diagnostic declaration contains an invalid HTTP status.
    InvalidStatusDeclaration(u16),
    /// Matching code omitted its required type identity.
    MissingType,
    /// Matching code was paired with another type URI.
    TypeMismatch {
        /// Type URI derived from the local declaration.
        expected: String,
        /// String-valued received type member.
        received: String,
    },
    /// Matching diagnostic arrived under the wrong transport status.
    StatusMismatch {
        /// Status fixed by the declaration.
        expected: StatusCode,
        /// Actual transport status.
        received: StatusCode,
    },
    /// Matching diagnostic omitted a declaration-required header.
    MissingRequiredHeader {
        /// Missing canonical header name.
        header: &'static str,
    },
    /// Matching Problem did not supply object-valued evidence.
    MissingEvidence,
    /// Evidence object did not decode into the declared public type.
    Evidence(serde_json::Error),
    /// A required response header did not satisfy its governed value contract.
    RequiredHeaderMismatch {
        /// Canonical lowercase header name.
        header: &'static str,
        /// Human-readable governed value contract.
        expected: &'static str,
    },
}

impl Display for TypedProblemError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDeclaration(error) => {
                write!(formatter, "invalid local declaration: {error}")
            }
            Self::InvalidStatusDeclaration(status) => {
                write!(formatter, "invalid local HTTP status declaration {status}")
            }
            Self::MissingType => formatter.write_str("matching diagnostic code omitted its type"),
            Self::TypeMismatch { expected, received } => write!(
                formatter,
                "diagnostic type mismatch: expected {expected}, received {received}"
            ),
            Self::StatusMismatch { expected, received } => write!(
                formatter,
                "diagnostic status mismatch: expected {expected}, received {received}"
            ),
            Self::MissingRequiredHeader { header } => write!(
                formatter,
                "diagnostic response omitted required header {header}"
            ),
            Self::RequiredHeaderMismatch { header, expected } => write!(
                formatter,
                "diagnostic response header {header} did not contain {expected}"
            ),
            Self::MissingEvidence => {
                formatter.write_str("matching diagnostic omitted object evidence")
            }
            Self::Evidence(error) => write!(formatter, "decode typed evidence: {error}"),
        }
    }
}

impl Error for TypedProblemError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDeclaration(error) => Some(error),
            Self::Evidence(error) => Some(error),
            Self::InvalidStatusDeclaration(_)
            | Self::MissingType
            | Self::TypeMismatch { .. }
            | Self::StatusMismatch { .. }
            | Self::MissingRequiredHeader { .. }
            | Self::RequiredHeaderMismatch { .. }
            | Self::MissingEvidence => None,
        }
    }
}
