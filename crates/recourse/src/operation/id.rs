//! Validated durable diagnostic occurrence identifiers.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Maximum UTF-8 bytes in an operation diagnostic identifier.
pub const MAX_OPERATION_DIAGNOSTIC_ID_BYTES: usize = 128;
const PREFIX: &str = "dia_";

/// Stable identifier for one durably recorded operation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct OperationDiagnosticId(Box<str>);

impl OperationDiagnosticId {
    /// Validates an application-generated identifier.
    pub fn try_new(value: impl Into<String>) -> Result<Self, OperationDiagnosticIdError> {
        let value = value.into();
        if value.len() > MAX_OPERATION_DIAGNOSTIC_ID_BYTES {
            return Err(OperationDiagnosticIdError::TooLong {
                maximum: MAX_OPERATION_DIAGNOSTIC_ID_BYTES,
                actual: value.len(),
            });
        }
        let Some(suffix) = value.strip_prefix(PREFIX) else {
            return Err(OperationDiagnosticIdError::InvalidPrefix);
        };
        if suffix.is_empty() {
            return Err(OperationDiagnosticIdError::EmptySuffix);
        }
        if !suffix.bytes().all(is_identifier_byte) {
            return Err(OperationDiagnosticIdError::InvalidCharacter);
        }
        Ok(Self(value.into_boxed_str()))
    }

    /// Returns the validated wire representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}

impl Display for OperationDiagnosticId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OperationDiagnosticId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

/// Rejected durable diagnostic identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OperationDiagnosticIdError {
    /// Identifier did not begin with `dia_`.
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

impl Display for OperationDiagnosticIdError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix => formatter.write_str("operation diagnostic ID must start dia_"),
            Self::EmptySuffix => formatter.write_str("operation diagnostic ID suffix is empty"),
            Self::InvalidCharacter => {
                formatter.write_str("operation diagnostic ID contains an invalid character")
            }
            Self::TooLong { maximum, actual } => write!(
                formatter,
                "operation diagnostic ID exceeds {maximum} bytes: {actual}"
            ),
        }
    }
}

impl Error for OperationDiagnosticIdError {}
