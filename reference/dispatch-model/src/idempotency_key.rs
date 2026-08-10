//! Bounded opaque idempotency identity accepted through an HTTP header.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

const MAX_IDEMPOTENCY_KEY_BYTES: usize = 128;

/// Nonempty visible-ASCII idempotency key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Validates a key without assigning meaning to its contents.
    pub fn new(value: impl Into<String>) -> Result<Self, IdempotencyKeyError> {
        let value = value.into();
        if value.is_empty() {
            return Err(IdempotencyKeyError::Empty);
        }
        if value.len() > MAX_IDEMPOTENCY_KEY_BYTES {
            return Err(IdempotencyKeyError::TooLong {
                actual_bytes: value.len(),
            });
        }
        if let Some((byte_index, byte)) = value
            .bytes()
            .enumerate()
            .find(|(_, byte)| !(b'!'..=b'~').contains(byte))
        {
            return Err(IdempotencyKeyError::InvalidByte { byte_index, byte });
        }
        Ok(Self(value))
    }

    /// Borrows the opaque validated key.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for IdempotencyKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Reason an idempotency key is unsafe to accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdempotencyKeyError {
    /// Key is empty.
    Empty,
    /// Key exceeds the public header budget.
    TooLong {
        /// Actual encoded byte length.
        actual_bytes: usize,
    },
    /// Key contains whitespace, control, or non-ASCII data.
    InvalidByte {
        /// Rejected byte index.
        byte_index: usize,
        /// Rejected byte.
        byte: u8,
    },
}

impl Display for IdempotencyKeyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("idempotency key must not be empty"),
            Self::TooLong { actual_bytes } => write!(
                formatter,
                "idempotency key is {actual_bytes} bytes; maximum is {MAX_IDEMPOTENCY_KEY_BYTES}"
            ),
            Self::InvalidByte { byte_index, byte } => write!(
                formatter,
                "idempotency key has invalid byte {byte:#04x} at index {byte_index}"
            ),
        }
    }
}

impl Error for IdempotencyKeyError {}
