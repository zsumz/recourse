//! Bounded public destination for one background dispatch job.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

const MAX_DESTINATION_BYTES: usize = 256;

/// Nonempty bounded destination label accepted by Dispatch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Destination(String);

impl Destination {
    /// Validates a caller-visible job destination.
    pub fn new(value: impl Into<String>) -> Result<Self, DestinationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(DestinationError::Empty);
        }
        if value.len() > MAX_DESTINATION_BYTES {
            return Err(DestinationError::TooLong {
                actual_bytes: value.len(),
            });
        }
        if let Some((character_index, _)) =
            value.chars().enumerate().find(|(_, ch)| ch.is_control())
        {
            return Err(DestinationError::ControlCharacter { character_index });
        }
        Ok(Self(value))
    }

    /// Borrows the validated destination.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Destination {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Destination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl JsonSchema for Destination {
    fn schema_name() -> Cow<'static, str> {
        "Destination".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "minLength": 1,
            "maxLength": MAX_DESTINATION_BYTES
        })
    }
}

/// Reason a Dispatch destination was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationError {
    /// Destination is empty.
    Empty,
    /// Encoded destination exceeds its public budget.
    TooLong {
        /// Actual UTF-8 byte length.
        actual_bytes: usize,
    },
    /// Destination contains terminal-unsafe control text.
    ControlCharacter {
        /// Unicode scalar index of the rejected character.
        character_index: usize,
    },
}

impl Display for DestinationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("destination must not be empty"),
            Self::TooLong { actual_bytes } => write!(
                formatter,
                "destination is {actual_bytes} bytes; maximum is {MAX_DESTINATION_BYTES}"
            ),
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "destination contains a control character at index {character_index}"
            ),
        }
    }
}

impl Error for DestinationError {}
