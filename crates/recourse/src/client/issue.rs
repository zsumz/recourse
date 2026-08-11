//! Nonfatal protocol inconsistencies retained beside tolerant input.

use http::StatusCode;

/// Protocol inconsistency that does not prevent fallback rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtocolIssue {
    /// A string-valued code was not canonical Recourse identity text.
    MalformedCode,
    /// String-valued durable diagnostic ID violated its syntax contract.
    MalformedOperationDiagnosticId,
    /// String-valued health finding ID violated its syntax contract.
    MalformedHealthFindingId,
    /// String-valued health severity was outside the closed vocabulary.
    InvalidHealthSeverity,
    /// String-valued health observation time was not valid RFC 3339.
    InvalidObservationTime,
    /// Numeric body status was not a valid HTTP status.
    InvalidBodyStatus,
    /// RFC 9457 body status disagreed with authoritative transport status.
    TransportStatusMismatch {
        /// Actual HTTP response status.
        transport: StatusCode,
        /// Valid but inconsistent body status.
        body: StatusCode,
    },
    /// Known code was paired with a different or missing type identity.
    UnexpectedTypeForCode {
        /// Type URI fixed by the local declaration.
        expected: String,
        /// String-valued received type, if present.
        received: Option<String>,
    },
    /// Authoritative transport status disagreed with the local declaration.
    CatalogStatusMismatch {
        /// Status fixed by the local declaration.
        expected: StatusCode,
        /// Actual HTTP response status.
        transport: StatusCode,
    },
    /// A response omitted a header required by its local declaration.
    MissingRequiredHeader {
        /// Canonical lowercase header name.
        header: String,
    },
    /// Known diagnostic code is not registered for the HTTP surface.
    CodeNotRegisteredForHttp,
    /// A known member was present with a JSON type outside its wire contract.
    InvalidMemberType {
        /// Canonical JSON member name.
        member: &'static str,
        /// Human-readable expected JSON shape.
        expected: &'static str,
    },
}
