//! Explicit validation boundary for dynamic caller-visible prose.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Default maximum encoded UTF-8 byte length for public prose.
pub const DEFAULT_PUBLIC_TEXT_BYTES: usize = 1_024;

/// Bounded caller-visible prose that contains no control characters.
///
/// Construction is an explicit review boundary. It does not attempt to detect
/// secrets and has no implicit conversion from `String` or error values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PublicText(Cow<'static, str>);

impl PublicText {
    /// Validates text against [`DEFAULT_PUBLIC_TEXT_BYTES`].
    pub fn new(value: impl Into<String>) -> Result<Self, PublicTextError> {
        Self::with_max_bytes(value, DEFAULT_PUBLIC_TEXT_BYTES)
    }

    /// Accepts a literal, rejecting invalid prose while the crate compiles.
    ///
    /// ```
    /// use recourse::diagnostic::PublicText;
    ///
    /// const DETAIL: PublicText = PublicText::from_static("Provide a destination.");
    /// assert_eq!(DETAIL.as_str(), "Provide a destination.");
    /// ```
    ///
    /// Invalid literals do not compile:
    ///
    /// ```compile_fail
    /// use recourse::diagnostic::PublicText;
    ///
    /// const EMPTY: PublicText = PublicText::from_static("");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when `value` is empty, exceeds [`DEFAULT_PUBLIC_TEXT_BYTES`], or
    /// contains a control character. Bound the result to a `const` item to turn
    /// those panics into compile errors, and use [`PublicText::new`] for text
    /// that arrives at runtime.
    pub const fn from_static(value: &'static str) -> Self {
        let bytes = value.as_bytes();
        assert!(!bytes.is_empty(), "public text must not be empty");
        assert!(
            bytes.len() <= DEFAULT_PUBLIC_TEXT_BYTES,
            "public text exceeds its encoded byte budget"
        );
        assert!(
            !contains_control_character(bytes),
            "public text must not contain a control character"
        );
        Self(Cow::Borrowed(value))
    }

    /// Validates text against an explicit encoded UTF-8 byte limit.
    pub fn with_max_bytes(
        value: impl Into<String>,
        max_bytes: usize,
    ) -> Result<Self, PublicTextError> {
        let value = value.into();
        if max_bytes == 0 {
            return Err(PublicTextError::ZeroLimit);
        }
        if value.is_empty() {
            return Err(PublicTextError::Empty);
        }
        if value.len() > max_bytes {
            return Err(PublicTextError::TooLong {
                actual_bytes: value.len(),
                max_bytes,
            });
        }
        if let Some((character_index, _)) =
            value.chars().enumerate().find(|(_, ch)| ch.is_control())
        {
            return Err(PublicTextError::ControlCharacter { character_index });
        }
        Ok(Self(Cow::Owned(value)))
    }

    /// Borrows the validated prose.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for PublicText {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for PublicText {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'de> Deserialize<'de> for PublicText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl JsonSchema for PublicText {
    fn schema_name() -> Cow<'static, str> {
        "PublicText".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "minLength": 1,
            "maxLength": DEFAULT_PUBLIC_TEXT_BYTES
        })
    }
}

/// Reports whether encoded bytes hold a character `char::is_control` accepts,
/// so literal constructors can run the same check in a `const` context.
///
/// C0 controls and `DEL` are single bytes; the C1 block is exactly the two-byte
/// sequences `0xc2 0x80` through `0xc2 0x9f`.
pub(crate) const fn contains_control_character(bytes: &[u8]) -> bool {
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte < 0x20 || byte == 0x7f {
            return true;
        }
        if byte == 0xc2 && index + 1 < bytes.len() && matches!(bytes[index + 1], 0x80..=0x9f) {
            return true;
        }
        index += 1;
    }
    false
}

/// Reason dynamic public prose was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicTextError {
    /// The configured byte limit is zero.
    ZeroLimit,
    /// Public prose is empty.
    Empty,
    /// Encoded UTF-8 exceeds its configured byte budget.
    TooLong {
        /// Actual encoded byte length.
        actual_bytes: usize,
        /// Configured maximum encoded byte length.
        max_bytes: usize,
    },
    /// Public prose contains a control character.
    ControlCharacter {
        /// Unicode scalar index of the first rejected character.
        character_index: usize,
    },
}

impl Display for PublicTextError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit => formatter.write_str("public text byte limit must be positive"),
            Self::Empty => formatter.write_str("public text must not be empty"),
            Self::TooLong {
                actual_bytes,
                max_bytes,
            } => write!(
                formatter,
                "public text is {actual_bytes} bytes; maximum is {max_bytes}"
            ),
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "public text contains a control character at index {character_index}"
            ),
        }
    }
}

impl Error for PublicTextError {}
