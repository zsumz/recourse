//! Replaceable generation of validated request-correlation identities.

use recourse::http::{CorrelationId, CorrelationIdError};

/// Generates request IDs that already satisfy Recourse's echo-safe profile.
pub trait RequestIdGenerator: Send + Sync + 'static {
    /// Generates one visible-ASCII, bounded correlation ID.
    fn generate(&self) -> Result<CorrelationId, CorrelationIdError>;
}

/// Monotonic ULID request-ID generator.
#[derive(Debug, Clone, Copy, Default)]
pub struct UlidRequestIds;

impl RequestIdGenerator for UlidRequestIds {
    fn generate(&self) -> Result<CorrelationId, CorrelationIdError> {
        CorrelationId::new(ulid::Ulid::new().to_string())
    }
}
