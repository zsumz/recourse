//! Canonical public identity for one Dispatch job.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

const PREFIX: &str = "job_";
const ULID_LENGTH: usize = 26;

/// Canonical `job_`-prefixed ULID assigned by Dispatch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct JobId(String);

impl JobId {
    /// Validates a canonical Dispatch job identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, JobIdError> {
        let value = value.into();
        let Some(suffix) = value.strip_prefix(PREFIX) else {
            return Err(JobIdError::InvalidPrefix);
        };
        if suffix.len() != ULID_LENGTH {
            return Err(JobIdError::InvalidLength {
                actual: suffix.len(),
            });
        }
        if !suffix.starts_with(|character: char| matches!(character, '0'..='7')) {
            return Err(JobIdError::InvalidCharacter {
                index: PREFIX.len(),
                byte: suffix.as_bytes()[0],
            });
        }
        if let Some((index, byte)) = suffix
            .bytes()
            .enumerate()
            .find(|(_, byte)| !is_crockford(*byte))
        {
            return Err(JobIdError::InvalidCharacter {
                index: index + PREFIX.len(),
                byte,
            });
        }
        Ok(Self(value))
    }

    /// Borrows the canonical identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for JobId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for JobId {
    type Err = JobIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for JobId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl JsonSchema for JobId {
    fn schema_name() -> Cow<'static, str> {
        "JobId".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": "^job_[0-7][0-9A-HJKMNP-TV-Z]{25}$"
        })
    }
}

/// Reason a purported Dispatch job identifier is not canonical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobIdError {
    /// Identifier does not begin with `job_`.
    InvalidPrefix,
    /// ULID suffix does not contain exactly 26 ASCII characters.
    InvalidLength {
        /// Actual encoded suffix length.
        actual: usize,
    },
    /// ULID suffix contains a byte outside uppercase Crockford Base32.
    InvalidCharacter {
        /// Byte index within the complete identifier.
        index: usize,
        /// Rejected byte.
        byte: u8,
    },
}

impl Display for JobIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix => formatter.write_str("job identifier must begin with 'job_'"),
            Self::InvalidLength { actual } => write!(
                formatter,
                "job identifier suffix has length {actual}; expected {ULID_LENGTH}"
            ),
            Self::InvalidCharacter { index, byte } => write!(
                formatter,
                "job identifier has invalid byte {byte:#04x} at index {index}"
            ),
        }
    }
}

impl Error for JobIdError {}

fn is_crockford(byte: u8) -> bool {
    byte.is_ascii_digit() || matches!(byte, b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'T' | b'V'..=b'Z')
}
