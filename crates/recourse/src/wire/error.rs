//! Stable wire-budget failure taxonomy.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

/// Diagnostic JSON resource budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireLimit {
    /// Encoded response body bytes.
    BodyBytes,
    /// Nested object and array depth.
    NestingDepth,
    /// Properties in one object.
    ObjectProperties,
    /// Items in one array.
    ArrayItems,
    /// UTF-8 bytes in one key or string.
    StringBytes,
    /// Items in the top-level suggestions array.
    Suggestions,
    /// Items in the validation violations array.
    Violations,
}

/// One shared protocol resource budget was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireLimitError {
    limit: WireLimit,
    maximum: usize,
    actual: usize,
}

impl WireLimitError {
    pub(crate) const fn new(limit: WireLimit, maximum: usize, actual: usize) -> Self {
        Self {
            limit,
            maximum,
            actual,
        }
    }

    /// Budget that rejected the value.
    pub const fn limit(&self) -> WireLimit {
        self.limit
    }

    /// Configured maximum.
    pub const fn maximum(&self) -> usize {
        self.maximum
    }

    /// Observed value, or the first observed size beyond a capped writer.
    pub const fn actual(&self) -> usize {
        self.actual
    }
}

impl Display for WireLimitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "diagnostic {:?} limit exceeded: maximum {}, actual {}",
            self.limit, self.maximum, self.actual
        )
    }
}

impl Error for WireLimitError {}
