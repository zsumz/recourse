//! Bounded lock decoding and semantic-validation failures.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::client::DecodeError;

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
