//! Typed delay-seconds and HTTP-date `Retry-After` policies.

use std::time::{Duration, SystemTime};

use http::{HeaderMap, HeaderValue, header::RETRY_AFTER};

use super::{HttpPolicy, PolicyError, sealed::Sealed};

/// Valid `Retry-After` delay-seconds or IMF-fixdate value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryAfter {
    /// Delay from response generation, rounded up to whole seconds.
    After(Duration),
    /// Absolute retry time rendered as an IMF-fixdate.
    At(SystemTime),
}

impl RetryAfter {
    /// Creates a delay-based retry value.
    pub const fn after(duration: Duration) -> Self {
        Self::After(duration)
    }

    /// Creates an absolute HTTP-date retry value.
    pub const fn at(time: SystemTime) -> Self {
        Self::At(time)
    }

    fn header_value(self) -> Result<HeaderValue, PolicyError> {
        let value = match self {
            Self::After(duration) => duration
                .as_secs()
                .saturating_add(u64::from(duration.subsec_nanos() > 0))
                .to_string(),
            Self::At(time) => httpdate::fmt_http_date(time),
        };
        HeaderValue::from_str(&value)
            .map_err(|_| PolicyError::new("Retry-After is not a header value"))
    }
}

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
