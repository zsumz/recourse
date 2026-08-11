//! Errors from bounded catalog artifact parsing and validation.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use crate::client::DecodeError;

/// Failure to decode or semantically validate a catalog artifact.
#[derive(Debug)]
#[non_exhaustive]
pub enum ArtifactParseError {
    /// JSON exceeded a resource limit or was malformed.
    Decode(DecodeError),
    /// JSON did not match the versioned artifact structure.
    Structure(serde_json::Error),
    /// Artifact uses a schema version this Recourse release cannot interpret.
    UnsupportedSchemaVersion {
        /// Unrecognized version.
        found: u32,
    },
    /// A compatibility-relevant artifact invariant was violated.
    Invalid {
        /// Precise artifact location.
        path: String,
        /// Actionable invariant description.
        reason: String,
    },
}

impl Display for ArtifactParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(formatter, "decode catalog artifact: {error}"),
            Self::Structure(error) => write!(formatter, "read catalog artifact structure: {error}"),
            Self::UnsupportedSchemaVersion { found } => {
                write!(
                    formatter,
                    "unsupported catalog artifact schema version {found}"
                )
            }
            Self::Invalid { path, reason } => {
                write!(formatter, "invalid catalog artifact at {path}: {reason}")
            }
        }
    }
}

impl Error for ArtifactParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(error) => Some(error),
            Self::Structure(error) => Some(error),
            Self::UnsupportedSchemaVersion { .. } | Self::Invalid { .. } => None,
        }
    }
}
