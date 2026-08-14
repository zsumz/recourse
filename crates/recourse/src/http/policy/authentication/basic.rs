//! RFC 7617 Basic challenges and their governed `401` policy.

use std::{
    borrow::Cow,
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::{HeaderMap, HeaderValue, header::WWW_AUTHENTICATE};

use super::{
    realm::{MAX_REALM_BYTES, RealmIssue, escape, is_visible_ascii, validate},
    response::has_valid_basic_challenge,
};
use crate::http::policy::{HttpPolicy, PolicyError, PolicyResponseIssue, sealed::Sealed};

/// Valid Basic challenge for a `WWW-Authenticate` response header.
///
/// RFC 7617 requires a realm and defines only one optional parameter:
/// `charset="UTF-8"`. Recourse always emits both values as quoted strings.
///
/// # Security
///
/// Basic credentials are only Base64-encoded, not encrypted. Applications
/// should use this challenge only over a TLS-protected connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicChallenge {
    realm: Cow<'static, str>,
    utf8: bool,
}

impl BasicChallenge {
    /// Creates a Basic challenge with a nonempty visible-ASCII realm.
    pub fn new(realm: &str) -> Result<Self, BasicChallengeError> {
        validate(realm).map_err(BasicChallengeError::from_issue)?;
        Ok(Self {
            realm: Cow::Owned(realm.to_owned()),
            utf8: false,
        })
    }

    /// Accepts a literal realm, rejecting invalid realms while compiling.
    ///
    /// # Panics
    ///
    /// Panics when `realm` is empty, exceeds its public header budget, or
    /// contains a byte outside visible ASCII. Bound the result to a `const`
    /// item to turn those panics into compile errors, and use
    /// [`BasicChallenge::new`] for a realm chosen at runtime.
    pub const fn from_static(realm: &'static str) -> Self {
        let bytes = realm.as_bytes();
        assert!(!bytes.is_empty(), "Basic realm must not be empty");
        assert!(
            bytes.len() <= MAX_REALM_BYTES,
            "Basic realm exceeds its public header budget"
        );
        let mut index = 0;
        while index < bytes.len() {
            assert!(
                is_visible_ascii(bytes[index]),
                "Basic realm must contain only visible ASCII"
            );
            index += 1;
        }
        Self {
            realm: Cow::Borrowed(realm),
            utf8: false,
        }
    }

    /// Advertises RFC 7617's advisory UTF-8 credential encoding.
    ///
    /// Servers using this parameter need to accept credentials encoded as
    /// UTF-8 after Unicode normalization form C.
    #[must_use]
    pub const fn with_utf8(mut self) -> Self {
        self.utf8 = true;
        self
    }

    fn header_value(&self) -> Result<HeaderValue, PolicyError> {
        let charset = if self.utf8 { ", charset=\"UTF-8\"" } else { "" };
        format!("Basic realm=\"{}\"{charset}", escape(&self.realm))
            .parse()
            .map_err(|_| PolicyError::new("Basic challenge is not a header value"))
    }
}

/// Reason a Basic challenge realm cannot be represented safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BasicChallengeError {
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

impl BasicChallengeError {
    fn from_issue(issue: RealmIssue) -> Self {
        match issue {
            RealmIssue::Empty => Self::EmptyRealm,
            RealmIssue::TooLong { actual_bytes } => Self::RealmTooLong { actual_bytes },
            RealmIssue::InvalidByte { byte_index, byte } => Self::InvalidByte { byte_index, byte },
        }
    }
}

impl Display for BasicChallengeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRealm => formatter.write_str("Basic realm must not be empty"),
            Self::RealmTooLong { actual_bytes } => write!(
                formatter,
                "Basic realm is {actual_bytes} bytes; maximum is {MAX_REALM_BYTES}"
            ),
            Self::InvalidByte { byte_index, byte } => write!(
                formatter,
                "Basic realm has invalid byte {byte:#04x} at index {byte_index}"
            ),
        }
    }
}

impl Error for BasicChallengeError {}

/// `401 Unauthorized` policy requiring a valid RFC 7617 Basic challenge.
#[derive(Debug, Clone, Copy, Default)]
pub struct BasicUnauthorized;

impl Sealed for BasicUnauthorized {
    fn validate_response_headers(headers: &HeaderMap) -> Result<(), PolicyResponseIssue> {
        if !headers.contains_key(WWW_AUTHENTICATE) || has_valid_basic_challenge(headers) {
            return Ok(());
        }
        Err(PolicyResponseIssue {
            header: "www-authenticate",
            expected: "a valid Basic challenge with a realm",
        })
    }
}

impl HttpPolicy for BasicUnauthorized {
    type Input = BasicChallenge;

    const STATUS: u16 = 401;
    const NAME: &'static str = "basic_unauthorized";
    const REQUIRED_HEADERS: &'static [&'static str] = &["www-authenticate"];

    fn headers(input: Self::Input) -> Result<HeaderMap, PolicyError> {
        let mut headers = HeaderMap::new();
        headers.insert(WWW_AUTHENTICATE, input.header_value()?);
        Ok(headers)
    }
}
