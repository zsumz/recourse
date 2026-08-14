//! Typed runtime inputs for policy-owned standard response headers.

use std::{
    borrow::Cow,
    collections::BTreeSet,
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::{HeaderMap, HeaderValue, Method, header::ALLOW};

use super::{HttpPolicy, PolicyError, sealed::Sealed};

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
#[non_exhaustive]
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
