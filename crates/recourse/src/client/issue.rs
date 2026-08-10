//! Nonfatal protocol inconsistencies retained beside tolerant input.

use http::StatusCode;

/// Protocol inconsistency that does not prevent fallback rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}
