//! Deterministic lock-encoding failures.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
};

/// Failure to write deterministic lock JSON.
#[derive(Debug)]
#[non_exhaustive]
pub enum LockWriteError {
    /// Lock serialization failed.
    Serialize(serde_json::Error),
    /// Destination rejected the encoded lock.
    Write(io::Error),
    /// Canonical output exceeded the catalog-lock body limit.
    TooLarge {
        /// Maximum accepted lock size in bytes.
        maximum: usize,
    },
}

impl Display for LockWriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "serialize catalog lock: {error}"),
            Self::Write(error) => write!(formatter, "write catalog lock: {error}"),
            Self::TooLarge { maximum } => {
                write!(formatter, "catalog lock exceeds {maximum} bytes")
            }
        }
    }
}

impl Error for LockWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
            Self::Write(error) => Some(error),
            Self::TooLarge { .. } => None,
        }
    }
}
