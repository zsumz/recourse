//! Actionable failures for malformed command-line intent.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArgumentError {
    MissingCommand,
    UnknownCommand(String),
    UnknownOption(String),
    MissingOption(&'static str),
    DuplicateOption(&'static str),
    MissingValue(&'static str),
    NonUtf8Option,
    NonUtf8Value(&'static str),
    InvalidFormat(String),
    InvalidNumber(String),
    InvalidCode(String),
}

impl Display for ArgumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommand => formatter.write_str("a command is required"),
            Self::UnknownCommand(value) => write!(formatter, "unknown command `{value}`"),
            Self::UnknownOption(value) => write!(formatter, "unknown option `{value}`"),
            Self::MissingOption(value) => write!(formatter, "required option `{value}` is missing"),
            Self::DuplicateOption(value) => write!(formatter, "option `{value}` was repeated"),
            Self::MissingValue(value) => write!(formatter, "option `{value}` requires a value"),
            Self::NonUtf8Option => formatter.write_str("option names must be valid UTF-8"),
            Self::NonUtf8Value(value) => write!(formatter, "value for `{value}` must be UTF-8"),
            Self::InvalidFormat(value) => write!(formatter, "unsupported output format `{value}`"),
            Self::InvalidNumber(value) => {
                write!(formatter, "invalid positive code number `{value}`")
            }
            Self::InvalidCode(value) => write!(formatter, "invalid diagnostic code `{value}`"),
        }
    }
}

impl Error for ArgumentError {}
