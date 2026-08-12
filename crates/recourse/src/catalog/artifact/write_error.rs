//! Deterministic artifact write failures.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
};

/// Error writing a deterministic catalog artifact.
#[derive(Debug)]
#[non_exhaustive]
pub enum ArtifactWriteError {
    /// JSON serialization failed.
    Serialize(serde_json::Error),
    /// The destination rejected the complete bounded output.
    Write(io::Error),
    /// Canonical output exceeded the catalog artifact body limit.
    TooLarge {
        /// Maximum accepted artifact size in bytes.
        maximum: usize,
    },
}

impl Display for ArtifactWriteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "serialize catalog artifact: {error}"),
            Self::Write(error) => write!(formatter, "write catalog artifact: {error}"),
            Self::TooLarge { maximum } => {
                write!(formatter, "catalog artifact exceeds {maximum} bytes")
            }
        }
    }
}

impl Error for ArtifactWriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) => Some(error),
            Self::Write(error) => Some(error),
            Self::TooLarge { .. } => None,
        }
    }
}
