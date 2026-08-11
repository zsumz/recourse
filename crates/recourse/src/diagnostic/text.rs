//! Explicit validation boundary for dynamic caller-visible prose.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Default maximum character length for public prose.
pub const DEFAULT_PUBLIC_TEXT_CHARS: usize = 1_024;

/// Bounded caller-visible prose that contains no control characters.
///
/// Construction is an explicit review boundary. It does not attempt to detect
/// secrets and has no implicit conversion from `String` or error values.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PublicText(Cow<'static, str>);

impl PublicText {
    /// Validates text against [`DEFAULT_PUBLIC_TEXT_CHARS`].
    pub fn new(value: impl Into<String>) -> Result<Self, PublicTextError> {
        Self::with_max_chars(value, DEFAULT_PUBLIC_TEXT_CHARS)
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
    /// Panics when `value` is empty, exceeds [`DEFAULT_PUBLIC_TEXT_CHARS`], or
    /// contains a control character. Bound the result to a `const` item to turn
    /// those panics into compile errors, and use [`PublicText::new`] for text
    /// that arrives at runtime.
    pub const fn from_static(value: &'static str) -> Self {
        let bytes = value.as_bytes();
        assert!(!bytes.is_empty(), "public text must not be empty");
        assert!(
            count_characters(bytes) <= DEFAULT_PUBLIC_TEXT_CHARS,
            "public text exceeds its character budget"
        );
        assert!(
            !contains_control_character(bytes),
            "public text must not contain a control character"
        );
        Self(Cow::Borrowed(value))
    }

    /// Validates text against an explicit character limit.
    pub fn with_max_chars(
        value: impl Into<String>,
        max_chars: usize,
    ) -> Result<Self, PublicTextError> {
        let value = value.into();
        if max_chars == 0 {
            return Err(PublicTextError::ZeroLimit);
        }
        if value.is_empty() {
            return Err(PublicTextError::Empty);
        }
        let actual_chars = value.chars().count();
        if actual_chars > max_chars {
            return Err(PublicTextError::TooLong {
                actual_chars,
                max_chars,
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
            "maxLength": DEFAULT_PUBLIC_TEXT_CHARS,
            "pattern": "^[^\\u0000-\\u001F\\u007F-\\u009F]*$"
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

/// Counts Unicode scalar values in a known-valid UTF-8 string from const code.
pub(crate) const fn count_characters(bytes: &[u8]) -> usize {
    let mut index = 0;
    let mut count = 0;
    while index < bytes.len() {
        if !matches!(bytes[index], 0x80..=0xbf) {
            count += 1;
        }
        index += 1;
    }
    count
}

/// Reason dynamic public prose was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublicTextError {
    /// The configured byte limit is zero.
    ZeroLimit,
    /// Public prose is empty.
    Empty,
    /// Text exceeds its configured character budget.
    TooLong {
        /// Actual character length.
        actual_chars: usize,
        /// Configured maximum character length.
        max_chars: usize,
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
            Self::ZeroLimit => formatter.write_str("public text character limit must be positive"),
            Self::Empty => formatter.write_str("public text must not be empty"),
            Self::TooLong {
                actual_chars,
                max_chars,
            } => write!(
                formatter,
                "public text is {actual_chars} characters; maximum is {max_chars}"
            ),
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "public text contains a control character at index {character_index}"
            ),
        }
    }
}

impl Error for PublicTextError {}
