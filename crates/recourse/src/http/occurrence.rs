//! Distinct request-correlation and RFC 9457 occurrence identities.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::Uri;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Maximum correlation ID byte length accepted at protocol boundaries.
pub const MAX_CORRELATION_ID_BYTES: usize = 128;

/// Bounded visible-ASCII request correlation value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct CorrelationId(String);

impl CorrelationId {
    /// Validates a value that can be echoed safely through an HTTP header.
    pub fn new(value: impl Into<String>) -> Result<Self, CorrelationIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(CorrelationIdError::Empty);
        }
        if value.len() > MAX_CORRELATION_ID_BYTES {
            return Err(CorrelationIdError::TooLong {
                actual_bytes: value.len(),
            });
        }
        if let Some((byte_index, byte)) = value
            .bytes()
            .enumerate()
            .find(|(_, byte)| !(b'!'..=b'~').contains(byte))
        {
            return Err(CorrelationIdError::InvalidByte { byte_index, byte });
        }
        Ok(Self(value))
    }

    /// Borrows the validated correlation value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CorrelationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CorrelationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl JsonSchema for CorrelationId {
    fn schema_name() -> Cow<'static, str> {
        "CorrelationId".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "minLength": 1,
            "maxLength": MAX_CORRELATION_ID_BYTES,
            "pattern": "^[!-~]+$"
        })
    }
}

/// Reason a request correlation value is unsafe to accept or echo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrelationIdError {
    /// Correlation value is empty.
    Empty,
    /// Encoded value exceeds its protocol budget.
    TooLong {
        /// Actual encoded byte length.
        actual_bytes: usize,
    },
    /// Value contains whitespace, control bytes, or non-ASCII data.
    InvalidByte {
        /// Index of the rejected encoded byte.
        byte_index: usize,
        /// Rejected byte.
        byte: u8,
    },
}

impl Display for CorrelationIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("correlation ID must not be empty"),
            Self::TooLong { actual_bytes } => write!(
                formatter,
                "correlation ID is {actual_bytes} bytes; maximum is {MAX_CORRELATION_ID_BYTES}"
            ),
            Self::InvalidByte { byte_index, byte } => write!(
                formatter,
                "correlation ID has invalid byte {byte:#04x} at index {byte_index}"
            ),
        }
    }
}

impl Error for CorrelationIdError {}

/// Identity shared by one HTTP Problem, its request, and operator telemetry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemOccurrence {
    correlation_id: CorrelationId,
    instance: Uri,
}

impl ProblemOccurrence {
    /// Combines a transport correlation ID with an absolute or relative URI reference.
    pub fn new(
        correlation_id: CorrelationId,
        instance: impl AsRef<str>,
    ) -> Result<Self, ProblemOccurrenceError> {
        let source = instance.as_ref();
        if source.is_empty() || source == "*" {
            return Err(ProblemOccurrenceError::InvalidInstance);
        }
        let instance = source
            .parse::<Uri>()
            .map_err(|_| ProblemOccurrenceError::InvalidInstance)?;
        Ok(Self {
            correlation_id,
            instance,
        })
    }

    /// Correlation value to echo through the configured request-ID header.
    pub const fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// RFC 9457 occurrence URI reference.
    pub const fn instance(&self) -> &Uri {
        &self.instance
    }
}

/// Reason a Problem occurrence could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemOccurrenceError {
    /// Instance is empty, `*`, or not a valid hierarchical URI reference.
    InvalidInstance,
}

impl Display for ProblemOccurrenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("Problem instance must be a valid nonempty URI reference")
    }
}

impl Error for ProblemOccurrenceError {}
