//! Typed runtime inputs for policy-owned standard response headers.

use std::{
    borrow::Cow,
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::{
    HeaderMap, HeaderValue, Method,
    header::{ALLOW, WWW_AUTHENTICATE},
};

use super::{HttpPolicy, PolicyError, sealed::Sealed};

const MAX_BEARER_REALM_BYTES: usize = 128;

/// Valid Bearer challenge for a `WWW-Authenticate` response header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerChallenge {
    realm: Cow<'static, str>,
}

impl BearerChallenge {
    /// Creates a Bearer challenge with a nonempty visible-ASCII realm.
    pub fn new(realm: &str) -> Result<Self, BearerChallengeError> {
        if realm.is_empty() {
            return Err(BearerChallengeError::EmptyRealm);
        }
        if realm.len() > MAX_BEARER_REALM_BYTES {
            return Err(BearerChallengeError::RealmTooLong {
                actual_bytes: realm.len(),
            });
        }
        if let Some((byte_index, byte)) = realm
            .bytes()
            .enumerate()
            .find(|(_, byte)| !is_visible_ascii(*byte))
        {
            return Err(BearerChallengeError::InvalidByte { byte_index, byte });
        }
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
            bytes.len() <= MAX_BEARER_REALM_BYTES,
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

    /// Escapes the reviewed realm into its canonical challenge value.
    fn header_value(&self) -> Result<HeaderValue, PolicyError> {
        let escaped = self.realm.replace('\\', "\\\\").replace('"', "\\\"");
        format!("Bearer realm=\"{escaped}\"")
            .parse()
            .map_err(|_| PolicyError::new("Bearer challenge is not a header value"))
    }
}

const fn is_visible_ascii(byte: u8) -> bool {
    matches!(byte, b' '..=b'~')
}

/// Reason a Bearer challenge realm cannot be represented safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl Display for BearerChallengeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRealm => formatter.write_str("Bearer realm must not be empty"),
            Self::RealmTooLong { actual_bytes } => {
                write!(
                    formatter,
                    "Bearer realm is {actual_bytes} bytes; maximum is {MAX_BEARER_REALM_BYTES}"
                )
            }
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

/// Nonempty deterministic set of methods for an `Allow` response header.
///
/// Equality compares the sorted, deduplicated set the header emits, so
/// declaration order and repetition never distinguish two values.
#[derive(Debug, Clone, Eq)]
pub struct AllowedMethods {
    methods: Cow<'static, [Method]>,
}

impl PartialEq for AllowedMethods {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_names() == other.canonical_names()
    }
}

impl AllowedMethods {
    /// Validates a nonempty set of methods chosen at runtime.
    pub fn new(methods: impl IntoIterator<Item = Method>) -> Result<Self, AllowedMethodsError> {
        let methods = methods.into_iter().collect::<Vec<_>>();
        if methods.is_empty() {
            return Err(AllowedMethodsError::Empty);
        }
        Ok(Self {
            methods: Cow::Owned(methods),
        })
    }

    /// Accepts a literal method set, rejecting an empty set while compiling.
    ///
    /// # Panics
    ///
    /// Panics when `methods` is empty. Bound the result to a `const` item to
    /// turn that panic into a compile error, and use [`AllowedMethods::new`]
    /// for a set assembled at runtime.
    pub const fn from_static(methods: &'static [Method]) -> Self {
        assert!(!methods.is_empty(), "allowed method set must not be empty");
        Self {
            methods: Cow::Borrowed(methods),
        }
    }

    /// Sorts and deduplicates the declared methods into the emitted set.
    fn canonical_names(&self) -> BTreeSet<&str> {
        self.methods.iter().map(Method::as_str).collect()
    }

    /// Joins the canonical method set into one `Allow` value.
    fn header_value(&self) -> Result<HeaderValue, PolicyError> {
        self.canonical_names()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ")
            .parse()
            .map_err(|_| PolicyError::new("allowed methods are not a header value"))
    }
}

/// Reason an allowed-method set cannot produce an `Allow` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedMethodsError {
    /// Method set is empty.
    Empty,
}

impl Display for AllowedMethodsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("allowed method set must not be empty"),
        }
    }
}

impl Error for AllowedMethodsError {}

/// `405 Method Not Allowed` policy requiring an `Allow` header.
#[derive(Debug, Clone, Copy, Default)]
pub struct MethodNotAllowed;

impl Sealed for MethodNotAllowed {}

impl HttpPolicy for MethodNotAllowed {
    type Input = AllowedMethods;

    const STATUS: u16 = 405;
    const NAME: &'static str = "method_not_allowed";
    const REQUIRED_HEADERS: &'static [&'static str] = &["allow"];

    fn headers(input: Self::Input) -> Result<HeaderMap, PolicyError> {
        let mut headers = HeaderMap::new();
        headers.insert(ALLOW, input.header_value()?);
        Ok(headers)
    }
}
