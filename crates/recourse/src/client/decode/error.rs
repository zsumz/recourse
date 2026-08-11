//! Stable bounded-input failure taxonomy.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Decode budget exceeded by untrusted input.
pub use crate::wire::WireLimit as DecodeLimit;

/// Malformed or resource-exhausting diagnostic input.
#[derive(Debug)]
pub enum DecodeError {
    /// Body is not valid JSON.
    MalformedJson(serde_json::Error),
    /// Valid JSON root is not an object.
    RootNotObject,
    /// One explicit resource budget was exceeded.
    LimitExceeded {
        /// Budget that rejected the input.
        limit: DecodeLimit,
        /// Configured maximum.
        maximum: usize,
        /// Observed value.
        actual: usize,
    },
}

impl Display for DecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson(error) => write!(formatter, "malformed diagnostic JSON: {error}"),
            Self::RootNotObject => formatter.write_str("diagnostic JSON root must be an object"),
            Self::LimitExceeded {
                limit,
                maximum,
                actual,
            } => write!(
                formatter,
                "diagnostic {limit:?} limit exceeded: maximum {maximum}, actual {actual}"
            ),
        }
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::MalformedJson(error) => Some(error),
            Self::RootNotObject | Self::LimitExceeded { .. } => None,
        }
    }
}
