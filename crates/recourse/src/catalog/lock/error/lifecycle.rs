//! Reservation, acceptance, and retirement mutation failures.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::catalog::{Code, CodeNumber};

use super::super::{CompatibilityReport, LockState, retirement::ReasonViolation};

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
    /// The lock namespace cannot represent every positive `u32` identity.
    TypeNamespaceTooLong {
        /// Maximum accepted type URI byte length.
        maximum: usize,
        /// Length required by the namespace's largest identity.
        actual: usize,
    },
    /// An internally generated candidate failed the public lock contract.
    InvalidGeneratedLock {
        /// Parser, writer, or semantic-closure failure.
        reason: String,
    },
}

impl Display for ReservationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeNamespaceTooLong { maximum, actual } => write!(
                formatter,
                "catalog type namespace requires {actual} bytes; maximum is {maximum}"
            ),
            Self::InvalidGeneratedLock { reason } => {
                write!(formatter, "generated catalog lock is invalid: {reason}")
            }
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
    /// An internally generated candidate failed the public lock contract.
    InvalidGeneratedLock {
        /// Parser, writer, or semantic-closure failure.
        reason: String,
    },
}

impl Display for AcceptanceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeneratedLock { reason } => {
                write!(formatter, "generated catalog lock is invalid: {reason}")
            }
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
    /// Retirement rationale exceeded the governed character limit.
    ReasonTooLong {
        /// Actual Unicode scalar-value count.
        actual_chars: usize,
        /// Maximum accepted Unicode scalar-value count.
        maximum: usize,
    },
    /// Retirement rationale contained a control character.
    ReasonControlCharacter {
        /// Zero-based character index.
        character_index: usize,
    },
    /// An internally generated candidate failed the public lock contract.
    InvalidGeneratedLock {
        /// Parser, writer, or semantic-closure failure.
        reason: String,
    },
}

impl Display for RetirementError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeneratedLock { reason } => {
                write!(formatter, "generated catalog lock is invalid: {reason}")
            }
            Self::UnknownCode { code } => write!(formatter, "diagnostic {code} is not locked"),
            Self::NotActive { code, state } => {
                write!(formatter, "diagnostic {code} cannot retire from {state:?}")
            }
            Self::EmptyReason => formatter.write_str("retirement reason must not be empty"),
            Self::ReasonTooLong {
                actual_chars,
                maximum,
            } => write!(
                formatter,
                "retirement reason is {actual_chars} characters; maximum is {maximum}"
            ),
            Self::ReasonControlCharacter { character_index } => write!(
                formatter,
                "retirement reason contains a control character at index {character_index}"
            ),
            Self::InvalidReplacement { code } => write!(
                formatter,
                "replacement {code} is not another active or retired diagnostic"
            ),
            Self::ReplacementCycle { code } => {
                write!(formatter, "replacement chain containing {code} would cycle")
            }
        }
    }
}

impl Error for RetirementError {}

impl From<ReasonViolation> for RetirementError {
    fn from(violation: ReasonViolation) -> Self {
        match violation {
            ReasonViolation::Empty => Self::EmptyReason,
            ReasonViolation::TooLong {
                actual_chars,
                maximum,
            } => Self::ReasonTooLong {
                actual_chars,
                maximum,
            },
            ReasonViolation::ControlCharacter { character_index } => {
                Self::ReasonControlCharacter { character_index }
            }
        }
    }
}
