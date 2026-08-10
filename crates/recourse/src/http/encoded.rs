//! Adapter-neutral canonical HTTP response parts.

use http::{HeaderMap, StatusCode};

/// Canonical status, headers, and JSON bytes ready for a framework adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedProblem {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl EncodedProblem {
    pub(crate) const fn new(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Actual HTTP response status.
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Canonical content type and policy-owned response headers.
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Canonical compact Problem JSON.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Splits the response into framework-neutral HTTP parts.
    pub fn into_parts(self) -> (StatusCode, HeaderMap, Vec<u8>) {
        (self.status, self.headers, self.body)
    }
}
