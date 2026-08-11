//! Exact owned RFC 3986 URI-reference values.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use fluent_uri::UriRef;

/// URI reference whose original encoded representation is preserved exactly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UriReference(String);

impl UriReference {
    /// Validates one nonempty RFC 3986 URI reference without normalization.
    pub fn new(value: impl Into<String>) -> Result<Self, UriReferenceError> {
        let value = value.into();
        if value.is_empty() {
            return Err(UriReferenceError::Empty);
        }
        UriRef::parse(value.as_str()).map_err(|_| UriReferenceError::Invalid)?;
        Ok(Self(value))
    }

    /// Borrows the exact validated representation supplied by the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for UriReference {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for UriReference {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Reason a value cannot serve as a nonempty URI reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UriReferenceError {
    /// Empty references are valid RFC 3986 references but not occurrence IDs.
    Empty,
    /// Value does not match the RFC 3986 URI-reference grammar.
    Invalid,
}

impl Display for UriReferenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("URI reference must not be empty"),
            Self::Invalid => formatter.write_str("value is not an RFC 3986 URI reference"),
        }
    }
}

impl Error for UriReferenceError {}
