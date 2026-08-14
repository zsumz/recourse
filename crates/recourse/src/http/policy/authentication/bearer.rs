//! Validated Bearer challenges and their governed `401` policy.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::{HeaderMap, HeaderValue, header::WWW_AUTHENTICATE};

use super::realm::{MAX_REALM_BYTES, RealmIssue, escape, is_visible_ascii, validate};
use crate::http::policy::{HttpPolicy, PolicyError, sealed::Sealed};

/// Valid Bearer challenge for a `WWW-Authenticate` response header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerChallenge {
    realm: Cow<'static, str>,
}

impl BearerChallenge {
    /// Creates a Bearer challenge with a nonempty visible-ASCII realm.
    pub fn new(realm: &str) -> Result<Self, BearerChallengeError> {
        validate(realm).map_err(BearerChallengeError::from_issue)?;
        Ok(Self {
            realm: Cow::Owned(realm.to_owned()),
        })
    }

    /// Accepts a literal realm, rejecting invalid realms while compiling.
    ///
    /// # Panics
    ///
    /// Panics when `realm` is empty, exceeds its public header budget, or
    /// contains a byte outside visible ASCII. Bound the result to a `const`
    /// item to turn those panics into compile errors, and use
    /// [`BearerChallenge::new`] for a realm chosen at runtime.
    pub const fn from_static(realm: &'static str) -> Self {
        let bytes = realm.as_bytes();
        assert!(!bytes.is_empty(), "Bearer realm must not be empty");
        assert!(
            bytes.len() <= MAX_REALM_BYTES,
            "Bearer realm exceeds its public header budget"
        );
        let mut index = 0;
        while index < bytes.len() {
            assert!(
                is_visible_ascii(bytes[index]),
                "Bearer realm must contain only visible ASCII"
            );
            index += 1;
        }
        Self {
            realm: Cow::Borrowed(realm),
        }
    }

    fn header_value(&self) -> Result<HeaderValue, PolicyError> {
        format!("Bearer realm=\"{}\"", escape(&self.realm))
            .parse()
            .map_err(|_| PolicyError::new("Bearer challenge is not a header value"))
    }
}

/// Reason a Bearer challenge realm cannot be represented safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BearerChallengeError {
    /// Realm is empty.
    EmptyRealm,
    /// Realm exceeds its public header budget.
    RealmTooLong {
        /// Actual encoded byte length.
        actual_bytes: usize,
    },
    /// Realm contains a control or non-ASCII byte.
    InvalidByte {
        /// Index of the rejected encoded byte.
        byte_index: usize,
        /// Rejected byte.
        byte: u8,
    },
}

impl BearerChallengeError {
    fn from_issue(issue: RealmIssue) -> Self {
        match issue {
            RealmIssue::Empty => Self::EmptyRealm,
            RealmIssue::TooLong { actual_bytes } => Self::RealmTooLong { actual_bytes },
            RealmIssue::InvalidByte { byte_index, byte } => Self::InvalidByte { byte_index, byte },
        }
    }
}

impl Display for BearerChallengeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRealm => formatter.write_str("Bearer realm must not be empty"),
            Self::RealmTooLong { actual_bytes } => write!(
                formatter,
                "Bearer realm is {actual_bytes} bytes; maximum is {MAX_REALM_BYTES}"
            ),
            Self::InvalidByte { byte_index, byte } => write!(
                formatter,
                "Bearer realm has invalid byte {byte:#04x} at index {byte_index}"
            ),
        }
    }
}

impl Error for BearerChallengeError {}

/// `401 Unauthorized` policy requiring a valid Bearer challenge.
#[derive(Debug, Clone, Copy, Default)]
pub struct BearerUnauthorized;

impl Sealed for BearerUnauthorized {}

impl HttpPolicy for BearerUnauthorized {
    type Input = BearerChallenge;

    const STATUS: u16 = 401;
    const NAME: &'static str = "unauthorized";
    const REQUIRED_HEADERS: &'static [&'static str] = &["www-authenticate"];

    fn headers(input: Self::Input) -> Result<HeaderMap, PolicyError> {
        let mut headers = HeaderMap::new();
        headers.insert(WWW_AUTHENTICATE, input.header_value()?);
        Ok(headers)
    }
}
