//! Bounded public names for non-body validation locations.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::header::HeaderName as HttpHeaderName;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::diagnostic::contains_control_character;

const MAX_PARAMETER_NAME_BYTES: usize = 128;
// Longest name `HeaderName::new` accepts, so no literal outruns its own type.
const MAX_FIELD_NAME_BYTES: usize = 65_535;

/// Bounded query or path parameter name without control characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ParameterName(Cow<'static, str>);

impl ParameterName {
    /// Validates a public parameter name.
    pub fn new(value: impl Into<String>) -> Result<Self, ParameterNameError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ParameterNameError::Empty);
        }
        if value.len() > MAX_PARAMETER_NAME_BYTES {
            return Err(ParameterNameError::TooLong {
                actual_bytes: value.len(),
            });
        }
        if let Some((character_index, _)) =
            value.chars().enumerate().find(|(_, ch)| ch.is_control())
        {
            return Err(ParameterNameError::ControlCharacter { character_index });
        }
        Ok(Self(Cow::Owned(value)))
    }

    /// Accepts a literal parameter name, rejecting invalid names while
    /// compiling.
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty, exceeds the public name budget, or
    /// contains a control character. Bound the result to a `const` item to turn
    /// those panics into compile errors, and use [`ParameterName::new`] for
    /// runtime input.
    pub const fn from_static(value: &'static str) -> Self {
        let bytes = value.as_bytes();
        assert!(!bytes.is_empty(), "parameter name must not be empty");
        assert!(
            bytes.len() <= MAX_PARAMETER_NAME_BYTES,
            "parameter name exceeds its encoded byte budget"
        );
        assert!(
            !contains_control_character(bytes),
            "parameter name must not contain a control character"
        );
        Self(Cow::Borrowed(value))
    }

    /// Borrows the validated name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ParameterName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ParameterName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl JsonSchema for ParameterName {
    fn schema_name() -> Cow<'static, str> {
        "ParameterName".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "minLength": 1,
            "maxLength": MAX_PARAMETER_NAME_BYTES
        })
    }
}

/// Reason a public query or path parameter name was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterNameError {
    /// Parameter name is empty.
    Empty,
    /// Encoded UTF-8 exceeds the public name budget.
    TooLong {
        /// Actual encoded byte length.
        actual_bytes: usize,
    },
    /// Parameter name contains a control character.
    ControlCharacter {
        /// Unicode scalar index of the rejected character.
        character_index: usize,
    },
}

impl Display for ParameterNameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("parameter name must not be empty"),
            Self::TooLong { actual_bytes } => write!(
                formatter,
                "parameter name is {actual_bytes} bytes; maximum is {MAX_PARAMETER_NAME_BYTES}"
            ),
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "parameter name contains a control character at index {character_index}"
            ),
        }
    }
}

impl Error for ParameterNameError {}

/// Canonical lowercase HTTP field name safe to expose without its value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct HeaderName(Cow<'static, str>);

impl HeaderName {
    /// Validates and canonicalizes an HTTP field name.
    pub fn new(value: &str) -> Result<Self, HeaderNameError> {
        let parsed = HttpHeaderName::from_bytes(value.as_bytes()).map_err(|_| HeaderNameError)?;
        Ok(Self(Cow::Owned(parsed.as_str().to_owned())))
    }

    /// Accepts a literal field name that is already canonical lowercase.
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty, exceeds the 65535-byte cap
    /// [`HeaderName::new`] also enforces, or contains a byte outside lowercase
    /// field-name token syntax. Canonicalization cannot allocate while
    /// compiling, so an uppercase literal is rejected rather than lowered.
    /// Bound the result to a `const` item to turn those panics into compile
    /// errors, and use [`HeaderName::new`] for runtime input.
    pub const fn from_static(value: &'static str) -> Self {
        let bytes = value.as_bytes();
        assert!(!bytes.is_empty(), "HTTP field name must not be empty");
        assert!(
            bytes.len() <= MAX_FIELD_NAME_BYTES,
            "HTTP field name exceeds its encoded byte budget"
        );
        let mut index = 0;
        while index < bytes.len() {
            assert!(
                is_canonical_field_byte(bytes[index]),
                "HTTP field name must be lowercase token syntax"
            );
            index += 1;
        }
        Self(Cow::Borrowed(value))
    }

    /// Borrows the canonical lowercase field name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for HeaderName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for HeaderName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(&value).map_err(D::Error::custom)
    }
}

impl JsonSchema for HeaderName {
    fn schema_name() -> Cow<'static, str> {
        "HeaderName".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": "^[!#$%&'*+.^_`|~0-9A-Za-z-]+$"
        })
    }
}

/// Token symbols RFC 9110 permits in a field name beside letters and digits.
const FIELD_NAME_SYMBOLS: &[u8] = b"!#$%&'*+-.^_`|~";

const fn is_canonical_field_byte(byte: u8) -> bool {
    if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
        return true;
    }
    let mut index = 0;
    while index < FIELD_NAME_SYMBOLS.len() {
        if FIELD_NAME_SYMBOLS[index] == byte {
            return true;
        }
        index += 1;
    }
    false
}

/// Error returned for invalid HTTP field-name syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeaderNameError;

impl Display for HeaderNameError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid HTTP field name")
    }
}

impl Error for HeaderNameError {}
