//! Bounded request method and normalized route metadata.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use http::Method;

/// Maximum normalized route byte length accepted for observation.
pub const MAX_NORMALIZED_ROUTE_BYTES: usize = 256;

/// Bounded route template such as `/jobs/{job_id}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizedRoute(String);

impl NormalizedRoute {
    /// Validates a normalized route template for safe telemetry use.
    pub fn new(value: impl Into<String>) -> Result<Self, NormalizedRouteError> {
        let value = value.into();
        if !value.starts_with('/') {
            return Err(NormalizedRouteError::MissingRoot);
        }
        if value.len() > MAX_NORMALIZED_ROUTE_BYTES {
            return Err(NormalizedRouteError::TooLong {
                actual_bytes: value.len(),
            });
        }
        if let Some((character_index, _)) =
            value.chars().enumerate().find(|(_, ch)| ch.is_control())
        {
            return Err(NormalizedRouteError::ControlCharacter { character_index });
        }
        Ok(Self(value))
    }

    /// Borrows the validated route template.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NormalizedRoute {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Reason a normalized telemetry route was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedRouteError {
    /// Route template does not begin at `/`.
    MissingRoot,
    /// Encoded route template exceeds its metadata budget.
    TooLong {
        /// Actual encoded byte length.
        actual_bytes: usize,
    },
    /// Route template contains a control character.
    ControlCharacter {
        /// Unicode scalar index of the rejected character.
        character_index: usize,
    },
}

impl Display for NormalizedRouteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRoot => formatter.write_str("normalized route must begin with '/'"),
            Self::TooLong { actual_bytes } => write!(
                formatter,
                "normalized route is {actual_bytes} bytes; maximum is {MAX_NORMALIZED_ROUTE_BYTES}"
            ),
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "normalized route contains a control character at index {character_index}"
            ),
        }
    }
}

impl Error for NormalizedRouteError {}

/// Optional bounded HTTP request metadata supplied by an adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpEventContext {
    method: Option<Method>,
    route: Option<NormalizedRoute>,
}

impl HttpEventContext {
    /// Creates empty adapter metadata.
    pub const fn new() -> Self {
        Self {
            method: None,
            route: None,
        }
    }

    /// Attaches a request method.
    #[must_use]
    pub fn with_method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Attaches a normalized route template.
    #[must_use]
    pub fn with_route(mut self, route: NormalizedRoute) -> Self {
        self.route = Some(route);
        self
    }

    /// Request method when known.
    pub const fn method(&self) -> Option<&Method> {
        self.method.as_ref()
    }

    /// Normalized route template when known.
    pub const fn route(&self) -> Option<&NormalizedRoute> {
        self.route.as_ref()
    }
}
