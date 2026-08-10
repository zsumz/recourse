//! Validated numeric identities and canonical prefixed codes.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

/// Positive numeric identity assigned permanently within one catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CodeNumber(u32);

impl CodeNumber {
    /// Creates a positive code number usable in diagnostic constants.
    ///
    /// # Panics
    ///
    /// Panics when `value` is zero. Use [`CodeNumber::try_new`] when the value
    /// comes from untrusted input.
    pub const fn new(value: u32) -> Self {
        assert!(value != 0, "a diagnostic code number must be positive");
        Self(value)
    }

    /// Validates a code number obtained at runtime.
    pub const fn try_new(value: u32) -> Result<Self, CodeNumberError> {
        if value == 0 {
            Err(CodeNumberError)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the positive integer representation.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for CodeNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl Serialize for CodeNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.get())
    }
}

impl<'de> Deserialize<'de> for CodeNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u32::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

/// Error returned when a numeric diagnostic identity is zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeNumberError;

impl Display for CodeNumberError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("a diagnostic code number must be positive")
    }
}

impl Error for CodeNumberError {}

/// Canonical one-to-one textual alias for a diagnostic type URI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code {
    prefix: Box<str>,
    number: CodeNumber,
}

impl Code {
    /// Combines a validated catalog prefix and positive number.
    pub fn new(prefix: &str, number: CodeNumber) -> Result<Self, CodeParseError> {
        validate_prefix(prefix)?;
        Ok(Self {
            prefix: prefix.into(),
            number,
        })
    }

    /// Returns the catalog prefix.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns the numeric identity within the catalog.
    pub const fn number(&self) -> CodeNumber {
        self.number
    }
}

impl Display for Code {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}-{}", self.prefix, self.number)
    }
}

impl FromStr for Code {
    type Err = CodeParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (prefix, number) = value
            .split_once('-')
            .ok_or(CodeParseError::MissingSeparator)?;
        validate_prefix(prefix)?;
        validate_number_text(number)?;
        let number = number.parse::<u32>().map_err(CodeParseError::Number)?;
        Self::new(prefix, CodeNumber::try_new(number)?)
    }
}

impl Serialize for Code {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Code {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = <Box<str>>::deserialize(deserializer)?;
        value.parse().map_err(D::Error::custom)
    }
}

/// Reason a textual diagnostic code is not canonical.
#[derive(Debug, PartialEq, Eq)]
pub enum CodeParseError {
    /// The code does not contain the required prefix-number separator.
    MissingSeparator,
    /// The catalog prefix violates its syntax contract.
    InvalidPrefix,
    /// The numeric portion is empty, padded, or contains nondigits.
    InvalidNumberSyntax,
    /// The numeric portion is outside the `u32` range.
    Number(ParseIntError),
    /// The numeric portion is zero.
    NumberValue(CodeNumberError),
}

impl Display for CodeParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSeparator => formatter.write_str("a code must contain '-'"),
            Self::InvalidPrefix => formatter.write_str("a code has an invalid catalog prefix"),
            Self::InvalidNumberSyntax => {
                formatter.write_str("a code number must be canonical decimal")
            }
            Self::Number(error) => write!(formatter, "a code number is invalid: {error}"),
            Self::NumberValue(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CodeParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Number(error) => Some(error),
            Self::NumberValue(error) => Some(error),
            _ => None,
        }
    }
}

impl From<CodeNumberError> for CodeParseError {
    fn from(error: CodeNumberError) -> Self {
        Self::NumberValue(error)
    }
}

fn validate_prefix(prefix: &str) -> Result<(), CodeParseError> {
    let mut bytes = prefix.bytes();
    let starts_with_letter = bytes.next().is_some_and(|byte| byte.is_ascii_uppercase());
    let valid_tail = bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
    if (2..=8).contains(&prefix.len()) && starts_with_letter && valid_tail {
        Ok(())
    } else {
        Err(CodeParseError::InvalidPrefix)
    }
}

fn validate_number_text(number: &str) -> Result<(), CodeParseError> {
    let canonical = !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
        && (number.len() == 1 || !number.starts_with('0'));
    canonical
        .then_some(())
        .ok_or(CodeParseError::InvalidNumberSyntax)
}
