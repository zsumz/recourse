//! Typed panic-free delay-seconds and HTTP-date `Retry-After` policies.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use http::{HeaderMap, HeaderValue, header::RETRY_AFTER};

use super::{HttpPolicy, PolicyError, sealed::Sealed};

const HTTP_DATE_UPPER_BOUND_SECONDS: u64 = 253_402_300_800;

/// Validated `Retry-After` delay-seconds or IMF-fixdate value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryAfter(RetryAfterValue);

#[derive(Debug, Clone, PartialEq, Eq)]
enum RetryAfterValue {
    After(Duration),
    At(String),
}

impl RetryAfter {
    /// Creates a delay-based retry value.
    pub const fn after(duration: Duration) -> Self {
        Self(RetryAfterValue::After(duration))
    }

    /// Validates and formats an absolute retry time without panicking.
    pub fn try_at(time: SystemTime) -> Result<Self, RetryAfterError> {
        let elapsed = time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RetryAfterError::BeforeUnixEpoch)?;
        if elapsed.as_secs() >= HTTP_DATE_UPPER_BOUND_SECONDS {
            return Err(RetryAfterError::AfterHttpDateRange);
        }
        Ok(Self(RetryAfterValue::At(httpdate::fmt_http_date(time))))
    }

    fn header_value(self) -> Result<HeaderValue, PolicyError> {
        let value = match self.0 {
            RetryAfterValue::After(duration) => duration
                .as_secs()
                .saturating_add(u64::from(duration.subsec_nanos() > 0))
                .to_string(),
            RetryAfterValue::At(value) => value,
        };
        HeaderValue::from_str(&value)
            .map_err(|_| PolicyError::new("Retry-After is not a header value"))
    }
}

/// Reason an absolute retry time cannot be represented as an HTTP-date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAfterError {
    /// HTTP dates cannot represent times before the Unix epoch.
    BeforeUnixEpoch,
    /// HTTP dates cannot represent the year 10000 or later.
    AfterHttpDateRange,
}

impl Display for RetryAfterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeUnixEpoch => formatter.write_str("retry time is before the Unix epoch"),
            Self::AfterHttpDateRange => {
                formatter.write_str("retry time is after the supported HTTP-date range")
            }
        }
    }
}

impl Error for RetryAfterError {}

/// `429` or `503` policy requiring a valid `Retry-After` header.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetryAfterPolicy<const STATUS: u16>;

impl Sealed for RetryAfterPolicy<429> {}
impl Sealed for RetryAfterPolicy<503> {}

impl HttpPolicy for RetryAfterPolicy<429> {
    type Input = RetryAfter;

    const STATUS: u16 = 429;
    const NAME: &'static str = "retry_after";
    const REQUIRED_HEADERS: &'static [&'static str] = &["retry-after"];

    fn headers(input: Self::Input) -> Result<HeaderMap, PolicyError> {
        retry_headers(input)
    }
}

impl HttpPolicy for RetryAfterPolicy<503> {
    type Input = RetryAfter;

    const STATUS: u16 = 503;
    const NAME: &'static str = "retry_after";
    const REQUIRED_HEADERS: &'static [&'static str] = &["retry-after"];

    fn headers(input: Self::Input) -> Result<HeaderMap, PolicyError> {
        retry_headers(input)
    }
}

fn retry_headers(input: RetryAfter) -> Result<HeaderMap, PolicyError> {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, input.header_value()?);
    Ok(headers)
}
