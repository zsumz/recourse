//! Failure to preserve a complete received document in rendered output.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// The renderer never drops remote data, so re-encoding it can fail.
#[derive(Debug)]
pub enum RenderError {
    /// Complete raw JSON could not be serialized.
    RawDocument(serde_json::Error),
}

impl Display for RenderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RawDocument(error) => write!(formatter, "serialize raw document: {error}"),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RawDocument(error) => Some(error),
        }
    }
}
