//! Metadata-only expected Problem and unexpected Fault events.

use http::StatusCode;

use crate::{
    catalog::Code,
    diagnostic::PublicEvidence,
    fault::Fault,
    http::{CorrelationId, Problem, ProblemOccurrence},
};

use super::{HttpEventContext, NormalizedRoute};

/// Protocol surface that emitted an observed diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventSurface {
    /// RFC 9457 HTTP Problem response.
    Http,
}

/// Bounded metadata for an expected caller-visible HTTP Problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemEvent {
    code: Code,
    surface: EventSurface,
    status: StatusCode,
    correlation_id: CorrelationId,
    request_method: Option<http::Method>,
    normalized_route: Option<NormalizedRoute>,
    fallback_encoding: bool,
}

impl ProblemEvent {
    /// Captures canonical HTTP metadata at an adapter fallback boundary.
    pub fn for_http(
        code: Code,
        status: StatusCode,
        occurrence: &ProblemOccurrence,
        context: &HttpEventContext,
        fallback_encoding: bool,
    ) -> Self {
        Self {
            code,
            surface: EventSurface::Http,
            status,
            correlation_id: occurrence.correlation_id().clone(),
            request_method: context.method().cloned(),
            normalized_route: context.route().cloned(),
            fallback_encoding,
        }
    }

    /// Captures metadata without retaining public evidence values.
    pub fn from_problem<E: PublicEvidence>(
        problem: &Problem<E>,
        context: &HttpEventContext,
        fallback_encoding: bool,
    ) -> Self {
        Self::for_http(
            problem.code().clone(),
            problem.status(),
            problem.occurrence(),
            context,
            fallback_encoding,
        )
    }

    /// Permanent diagnostic code.
    pub const fn code(&self) -> &Code {
        &self.code
    }

    /// Emitting protocol surface.
    pub const fn surface(&self) -> EventSurface {
        self.surface
    }

    /// Actual HTTP response status.
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// High-cardinality request correlation value for logs or traces.
    pub const fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// Request method when supplied by the adapter.
    pub const fn request_method(&self) -> Option<&http::Method> {
        self.request_method.as_ref()
    }

    /// Normalized route template when supplied by the adapter.
    pub const fn normalized_route(&self) -> Option<&NormalizedRoute> {
        self.normalized_route.as_ref()
    }

    /// Whether an integration boundary emitted a sanitized fallback.
    pub const fn used_fallback_encoding(&self) -> bool {
        self.fallback_encoding
    }
}

/// Bounded metadata for an unexpected fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEvent(ProblemEvent);

impl FaultEvent {
    /// Marks canonical HTTP metadata as an unexpected integration failure.
    pub fn for_http(
        code: Code,
        status: StatusCode,
        occurrence: &ProblemOccurrence,
        context: &HttpEventContext,
        fallback_encoding: bool,
    ) -> Self {
        Self(ProblemEvent::for_http(
            code,
            status,
            occurrence,
            context,
            fallback_encoding,
        ))
    }

    /// Captures fault metadata without retaining evidence or private reports.
    pub fn from_fault<E: PublicEvidence>(
        fault: &Fault<E>,
        context: &HttpEventContext,
        fallback_encoding: bool,
    ) -> Self {
        Self(ProblemEvent::from_problem(
            fault.problem(),
            context,
            fallback_encoding,
        ))
    }

    /// Shared bounded HTTP metadata.
    pub const fn problem_metadata(&self) -> &ProblemEvent {
        &self.0
    }
}
