//! Per-request correlation, occurrence, and bounded route preparation.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use axum::{body::Body, extract::MatchedPath};
use http::Request;
use recourse::{
    http::{CorrelationId, CorrelationIdError, ProblemOccurrence, ProblemOccurrenceError},
    observe::{HttpEventContext, NormalizedRoute},
};

use crate::{
    ProblemContext,
    layer::LayerConfig,
    request_id::{RequestIdGenerator, UlidRequestIds},
};

pub(crate) fn prepare<C: recourse::catalog::CatalogSpec>(
    request: &Request<Body>,
    config: &LayerConfig<C>,
) -> Result<(ProblemContext<C>, CorrelationId), ScopeError> {
    let correlation_id = incoming_id(request, config)
        .map_or_else(|| config.request_ids.generate(), Ok)
        .map_err(ScopeError::RequestId)?;
    let instance = (config.instance_uri)(&correlation_id);
    let occurrence = ProblemOccurrence::new(correlation_id.clone(), instance)
        .map_err(ScopeError::ProblemOccurrence)?;
    let context = ProblemContext::new(config.runtime.clone(), occurrence, event_context(request));
    Ok((context, correlation_id))
}

pub(crate) fn event_context(request: &Request<Body>) -> HttpEventContext {
    let context = HttpEventContext::new().with_method(request.method().clone());
    request
        .extensions()
        .get::<MatchedPath>()
        .and_then(|path| NormalizedRoute::new(path.as_str()).ok())
        .map_or(context.clone(), |route| context.with_route(route))
}

pub(crate) fn emergency_occurrence() -> Option<ProblemOccurrence> {
    let correlation_id = UlidRequestIds.generate().ok()?;
    let instance = format!("/problem-occurrences/{correlation_id}");
    ProblemOccurrence::new(correlation_id, instance).ok()
}

fn incoming_id<C: recourse::catalog::CatalogSpec>(
    request: &Request<Body>,
    config: &LayerConfig<C>,
) -> Option<CorrelationId> {
    request
        .headers()
        .get(&config.request_id_header)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| CorrelationId::new(value).ok())
}

/// Request-scoping invariant failure sent through the internal fallback.
#[derive(Debug)]
pub(crate) enum ScopeError {
    RequestId(CorrelationIdError),
    ProblemOccurrence(ProblemOccurrenceError),
}

impl Display for ScopeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestId(error) => write!(formatter, "generate request ID: {error}"),
            Self::ProblemOccurrence(error) => {
                write!(formatter, "generate Problem instance: {error}")
            }
        }
    }
}

impl Error for ScopeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RequestId(error) => Some(error),
            Self::ProblemOccurrence(error) => Some(error),
        }
    }
}
