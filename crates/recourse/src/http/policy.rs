//! Sealed policy metadata that keeps status selection catalog-governed.

mod authentication;
mod header;
mod mandatory;
mod retry;

use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
};

use http::HeaderMap;

use crate::diagnostic::DiagnosticType;

pub use authentication::{
    BasicChallenge, BasicChallengeError, BasicUnauthorized, BearerChallenge, BearerChallengeError,
    BearerUnauthorized,
};
pub use header::{AllowedMethods, AllowedMethodsError, MethodNotAllowed};
pub(crate) use mandatory::mandatory_headers;
pub use retry::{RetryAfter, RetryAfterError, RetryAfterPolicy};

mod sealed {
    //! Private boundary for protocol-owned HTTP policies.

    use http::HeaderMap;

    use super::PolicyResponseIssue;

    pub trait Sealed {
        fn validate_response_headers(_headers: &HeaderMap) -> Result<(), PolicyResponseIssue> {
            Ok(())
        }
    }
}

/// Internal mismatch between a received header and a sealed policy contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyResponseIssue {
    /// Canonical required header name.
    pub(crate) header: &'static str,
    /// Human-readable governed value contract.
    pub(crate) expected: &'static str,
}

pub(crate) fn validate_typed_response_headers<P: HttpPolicy>(
    headers: &HeaderMap,
) -> Result<(), PolicyResponseIssue> {
    <P as sealed::Sealed>::validate_response_headers(headers)
}

pub(crate) fn validate_named_response_headers(
    policy: &str,
    headers: &HeaderMap,
) -> Result<(), PolicyResponseIssue> {
    if policy == BasicUnauthorized::NAME {
        validate_typed_response_headers::<BasicUnauthorized>(headers)
    } else if policy == BearerUnauthorized::NAME {
        validate_typed_response_headers::<BearerUnauthorized>(headers)
    } else {
        Ok(())
    }
}

/// Protocol-owned HTTP response policy.
///
/// The trait is sealed in `0.0.1` so status, headers, body, and artifact metadata
/// cannot disagree through third-party policy implementations.
///
/// Application-defined policies cannot cross the sealed boundary:
///
/// ```compile_fail
/// use http::HeaderMap;
/// use recourse::http::{HttpPolicy, PolicyError};
///
/// #[derive(Debug)]
/// struct ApplicationPolicy;
///
/// impl HttpPolicy for ApplicationPolicy {
///     type Input = ();
///     const STATUS: u16 = 499;
///     const NAME: &'static str = "application";
///     const REQUIRED_HEADERS: &'static [&'static str] = &[];
///
///     fn headers(_: ()) -> Result<HeaderMap, PolicyError> {
///         Ok(HeaderMap::new())
///     }
/// }
/// ```
pub trait HttpPolicy: sealed::Sealed + Debug + Send + Sync + 'static {
    /// Typed runtime input required to construct policy-owned headers.
    type Input: Debug + Send + Sync + 'static;

    /// Status code emitted by the policy.
    const STATUS: u16;

    /// Stable artifact name for the policy family.
    const NAME: &'static str;

    /// Response headers the policy always requires.
    const REQUIRED_HEADERS: &'static [&'static str];

    /// Constructs and validates policy-owned response headers.
    fn headers(input: Self::Input) -> Result<HeaderMap, PolicyError>;
}

/// Fixed HTTP status policy with no policy-specific required headers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Fixed<const STATUS: u16>;

impl<const STATUS: u16> sealed::Sealed for Fixed<STATUS> {}

impl<const STATUS: u16> HttpPolicy for Fixed<STATUS> {
    type Input = ();

    const STATUS: u16 = STATUS;
    const NAME: &'static str = "fixed";
    const REQUIRED_HEADERS: &'static [&'static str] = &[];

    fn headers(_input: Self::Input) -> Result<HeaderMap, PolicyError> {
        Ok(HeaderMap::new())
    }
}

/// Declares that a diagnostic is available as an HTTP Problem.
pub trait HttpProblemType: DiagnosticType {
    /// Governed status and header policy for this diagnostic.
    type Policy: HttpPolicy;
}

/// Invalid runtime input supplied to a built-in HTTP policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyError {
    message: &'static str,
}

impl PolicyError {
    pub(super) const fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Display for PolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.message)
    }
}

impl Error for PolicyError {}
