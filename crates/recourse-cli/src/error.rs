//! Explicit filesystem and artifact failures from command execution.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::PathBuf,
};

use recourse::catalog::{ArtifactParseError, LockParseError, LockWriteError, RetirementError};

#[derive(Debug)]
pub(crate) enum CommandError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    InputTooLarge {
        path: PathBuf,
        maximum: usize,
    },
    ParseArtifact {
        path: PathBuf,
        source: ArtifactParseError,
    },
    ParseLock {
        path: PathBuf,
        source: LockParseError,
    },
    EncodeLock(LockWriteError),
    Retire(RetirementError),
    Write {
        path: PathBuf,
        source: io::Error,
    },
    InvalidManifest {
        path: PathBuf,
        entry: String,
    },
    UnsafeDocumentation {
        path: PathBuf,
        reason: &'static str,
    },
    Json(serde_json::Error),
}

impl Display for CommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "read `{}`: {source}", path.display())
            }
            Self::InputTooLarge { path, maximum } => write!(
                formatter,
                "read `{}`: input exceeds the {maximum}-byte limit",
                path.display()
            ),
            Self::ParseArtifact { path, source } => {
                write!(formatter, "parse catalog `{}`: {source}", path.display())
            }
            Self::ParseLock { path, source } => {
                write!(formatter, "parse lock `{}`: {source}", path.display())
            }
            Self::EncodeLock(source) => write!(formatter, "encode catalog lock: {source}"),
            Self::Retire(source) => write!(formatter, "retire diagnostic: {source}"),
            Self::Write { path, source } => {
                write!(formatter, "write `{}`: {source}", path.display())
            }
            Self::InvalidManifest { path, entry } => write!(
                formatter,
                "generated-doc manifest `{}` contains unsafe path `{entry}`",
                path.display()
            ),
            Self::UnsafeDocumentation { path, reason } => {
                write!(
                    formatter,
                    "unsafe documentation path `{}`: {reason}",
                    path.display()
                )
            }
            Self::Json(source) => write!(formatter, "encode JSON output: {source}"),
        }
    }
}

impl Error for CommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::ParseArtifact { source, .. } => Some(source),
            Self::ParseLock { source, .. } => Some(source),
            Self::EncodeLock(source) => Some(source),
            Self::Retire(source) => Some(source),
            Self::Json(source) => Some(source),
            Self::InputTooLarge { .. }
            | Self::InvalidManifest { .. }
            | Self::UnsafeDocumentation { .. } => None,
        }
    }
}

impl CommandError {
    pub(crate) fn stdout(source: io::Error) -> Self {
        Self::Write {
            path: "<stdout>".into(),
            source,
        }
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}
