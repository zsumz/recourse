//! Validated RFC 6901 locations for public request-body evidence.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::diagnostic::contains_control_character;

/// Valid RFC 6901 JSON Pointer without terminal control characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct JsonPointer(Cow<'static, str>);

impl JsonPointer {
    /// Validates an RFC 6901 JSON Pointer.
    pub fn new(value: impl Into<String>) -> Result<Self, JsonPointerError> {
        let value = value.into();
        if !value.is_empty() && !value.starts_with('/') {
            return Err(JsonPointerError::MissingRootSeparator);
        }
        if let Some(byte_index) = invalid_escape(value.as_bytes()) {
            return Err(JsonPointerError::InvalidEscape { byte_index });
        }
        if let Some((character_index, _)) =
            value.chars().enumerate().find(|(_, ch)| ch.is_control())
        {
            return Err(JsonPointerError::ControlCharacter { character_index });
        }
        Ok(Self(Cow::Owned(value)))
    }

    /// Accepts a literal pointer, rejecting invalid syntax while compiling.
    ///
    /// ```
    /// use recourse::validation::JsonPointer;
    ///
    /// const DESTINATION: JsonPointer = JsonPointer::from_static("/destination");
    /// assert_eq!(DESTINATION.as_str(), "/destination");
    /// ```
    ///
    /// Invalid literals do not compile:
    ///
    /// ```compile_fail
    /// use recourse::validation::JsonPointer;
    ///
    /// const RELATIVE: JsonPointer = JsonPointer::from_static("destination");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when `value` is a nonempty pointer that does not begin with `/`,
    /// contains an escape other than `~0` or `~1`, or contains a control
    /// character. Bound the result to a `const` item to turn those panics into
    /// compile errors, and use [`JsonPointer::new`] for runtime input.
    pub const fn from_static(value: &'static str) -> Self {
        let bytes = value.as_bytes();
        assert!(
            bytes.is_empty() || bytes[0] == b'/',
            "JSON Pointer must begin with '/'"
        );
        assert!(
            invalid_escape(bytes).is_none(),
            "JSON Pointer must escape '~' as '~0' or '~1'"
        );
        assert!(
            !contains_control_character(bytes),
            "JSON Pointer must not contain a control character"
        );
        Self(Cow::Borrowed(value))
    }

    /// Borrows the canonical pointer text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for JsonPointer {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for JsonPointer {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'de> Deserialize<'de> for JsonPointer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl JsonSchema for JsonPointer {
    fn schema_name() -> Cow<'static, str> {
        "JsonPointer".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": "^(?:/(?:[^\\u0000-\\u001F\\u007F-\\u009F~]|~[01])*)*$"
        })
    }
}

/// Reason a request-body location is not a valid public JSON Pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum JsonPointerError {
    /// A non-root pointer does not begin with `/`.
    MissingRootSeparator,
    /// A `~` escape is not followed by `0` or `1`.
    InvalidEscape {
        /// Encoded UTF-8 byte index of the rejected `~`.
        byte_index: usize,
    },
    /// Pointer contains a terminal control character.
    ControlCharacter {
        /// Unicode scalar index of the rejected character.
        character_index: usize,
    },
}

impl Display for JsonPointerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRootSeparator => formatter.write_str("JSON Pointer must begin with '/'"),
            Self::InvalidEscape { byte_index } => {
                write!(
                    formatter,
                    "JSON Pointer has an invalid escape at byte {byte_index}"
                )
            }
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "JSON Pointer contains a control character at index {character_index}"
            ),
        }
    }
}

impl Error for JsonPointerError {}

const fn invalid_escape(bytes: &[u8]) -> Option<usize> {
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~'
            && (index + 1 == bytes.len() || !matches!(bytes[index + 1], b'0' | b'1'))
        {
            return Some(index);
        }
        index += 1;
    }
    None
}
