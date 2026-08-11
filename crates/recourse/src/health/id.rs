//! Validated current health-finding identifiers.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Maximum UTF-8 bytes in a health finding identifier.
pub const MAX_HEALTH_FINDING_ID_BYTES: usize = 128;
const PREFIX: &str = "finding_";

/// Stable identifier for one observed health finding.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct HealthFindingId(Box<str>);

impl HealthFindingId {
    /// Validates an application-generated finding identifier.
    pub fn try_new(value: impl Into<String>) -> Result<Self, HealthFindingIdError> {
        let value = value.into();
        if value.len() > MAX_HEALTH_FINDING_ID_BYTES {
            return Err(HealthFindingIdError::TooLong {
                maximum: MAX_HEALTH_FINDING_ID_BYTES,
                actual: value.len(),
            });
        }
        let Some(suffix) = value.strip_prefix(PREFIX) else {
            return Err(HealthFindingIdError::InvalidPrefix);
        };
        if suffix.is_empty() {
            return Err(HealthFindingIdError::EmptySuffix);
        }
        if !suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(HealthFindingIdError::InvalidCharacter);
        }
        Ok(Self(value.into_boxed_str()))
    }

    /// Returns the validated wire representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for HealthFindingId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HealthFindingId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

/// Rejected health finding identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HealthFindingIdError {
    /// Identifier did not begin with `finding_`.
    InvalidPrefix,
    /// Identifier contained no application-generated suffix.
    EmptySuffix,
    /// Suffix contained a character outside ASCII letters, digits, `_`, and `-`.
    InvalidCharacter,
    /// Identifier exceeded the protocol byte budget.
    TooLong {
        /// Maximum permitted UTF-8 bytes.
        maximum: usize,
        /// Actual UTF-8 bytes.
        actual: usize,
    },
}

impl Display for HealthFindingIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix => formatter.write_str("health finding ID must start finding_"),
            Self::EmptySuffix => formatter.write_str("health finding ID suffix is empty"),
            Self::InvalidCharacter => {
                formatter.write_str("health finding ID contains an invalid character")
            }
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "health finding ID exceeds {maximum} bytes: {actual}"
            ),
        }
    }
}

impl Error for HealthFindingIdError {}
