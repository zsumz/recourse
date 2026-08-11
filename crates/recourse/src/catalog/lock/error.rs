//! Parse, write, and reservation failures for catalog locks.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
};

use crate::{
    catalog::{Code, CodeNumber},
    client::DecodeError,
};

use super::{CompatibilityReport, LockState};

/// Failure to decode or semantically validate a catalog lock.
#[derive(Debug)]
#[non_exhaustive]
pub enum LockParseError {
    /// JSON exceeded a resource limit or was malformed.
    Decode(DecodeError),
    /// JSON did not match the versioned lock structure.
    Structure(serde_json::Error),
    /// Lock uses a schema version this release cannot interpret.
    UnsupportedSchemaVersion {
        /// Unrecognized version.
        found: u32,
    },
    /// A permanent lock invariant was violated.
    Invalid {
        /// Precise lock location.
        path: String,
        /// Actionable invariant description.
        reason: String,
    },
}

impl Display for LockParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "decode catalog lock: {error}"),
            Self::Structure(error) => write!(formatter, "read catalog lock structure: {error}"),
            Self::UnsupportedSchemaVersion { found } => {
                write!(formatter, "unsupported catalog lock schema version {found}")
            }
            Self::Invalid { path, reason } => {
                write!(formatter, "invalid catalog lock at {path}: {reason}")
            }
        }
    }
}

impl Error for LockParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Structure(error) => Some(error),
            Self::UnsupportedSchemaVersion { .. } | Self::Invalid { .. } => None,
        }
    }
}

/// Failure to write deterministic lock JSON.
#[derive(Debug)]
#[non_exhaustive]
pub enum LockWriteError {
    /// Lock serialization failed.
    Serialize(serde_json::Error),
    /// Destination rejected the encoded lock.
    Write(io::Error),
}

impl Display for LockWriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "serialize catalog lock: {error}"),
            Self::Write(error) => write!(formatter, "write catalog lock: {error}"),
        }
    }
}

impl Error for LockWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
            Self::Write(error) => Some(error),
        }
    }
}

/// Refused reservation of a permanent diagnostic number.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReservationError {
    /// Explicit number already appears anywhere in lock history.
    AlreadyUsed {
        /// Rejected permanent number.
        number: CodeNumber,
    },
    /// No larger `u32` identity can be allocated.
    NumberSpaceExhausted,
    /// Validated lock prefix unexpectedly failed code construction.
    InvalidLockPrefix,
}

impl Display for ReservationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyUsed { number } => write!(formatter, "diagnostic number {number} is used"),
            Self::NumberSpaceExhausted => formatter.write_str("diagnostic number space exhausted"),
            Self::InvalidLockPrefix => formatter.write_str("catalog lock prefix is invalid"),
        }
    }
}

impl Error for ReservationError {}

/// Refused acceptance of a catalog compatibility report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcceptanceError {
    /// Permanent identity or tombstone history would be violated.
    Forbidden(CompatibilityReport),
    /// Breaking changes were not explicitly acknowledged.
    BreakingRequiresAcknowledgement(CompatibilityReport),
}

impl Display for AcceptanceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forbidden(report) => write!(
                formatter,
                "catalog has {} forbidden compatibility change(s)",
                report.changes().len()
            ),
            Self::BreakingRequiresAcknowledgement(report) => write!(
                formatter,
                "catalog has {} change(s), including unacknowledged breaks",
                report.changes().len()
            ),
        }
    }
}

impl Error for AcceptanceError {}

/// Refused explicit transition from active definition to tombstone.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RetirementError {
    /// Code is absent from lock history.
    UnknownCode {
        /// Missing code.
        code: Code,
    },
    /// Only an active entry can transition to retired.
    NotActive {
        /// Code in the wrong lifecycle state.
        code: Code,
        /// Current state.
        state: LockState,
    },
    /// Retirement rationale was empty or whitespace-only.
    EmptyReason,
    /// Replacement was the retiring code, absent, or only reserved.
    InvalidReplacement {
        /// Rejected replacement code.
        code: Code,
    },
    /// The retirement would introduce a replacement cycle.
    ReplacementCycle {
        /// Code participating in the rejected cycle.
        code: Code,
    },
}

impl Display for RetirementError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCode { code } => write!(formatter, "diagnostic {code} is not locked"),
            Self::NotActive { code, state } => {
                write!(formatter, "diagnostic {code} cannot retire from {state:?}")
            }
            Self::EmptyReason => formatter.write_str("retirement reason must not be empty"),
            Self::InvalidReplacement { code } => {
                write!(
                    formatter,
                    "replacement {code} is not another active or retired diagnostic"
                )
            }
            Self::ReplacementCycle { code } => {
                write!(formatter, "replacement chain containing {code} would cycle")
            }
        }
    }
}

impl Error for RetirementError {}
