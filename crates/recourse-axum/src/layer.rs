//! Tower request lifecycle integration and request-ID response echo.

use std::{
    convert::Infallible,
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use futures_util::FutureExt;
use http::{HeaderName, HeaderValue, Request};
use recourse::{
    catalog::{Catalog, CatalogSpec},
    fault::PrivateReport,
    http::CorrelationId,
};
use tower::{Layer, Service};

use crate::{
    builder::RecourseLayerBuilder,
    panic::RecoveredPanic,
    request_id::RequestIdGenerator,
    runtime::{Runtime, private_report},
    scope,
};

pub(crate) type InstanceFactory = dyn Fn(&CorrelationId) -> String + Send + Sync + 'static;

pub(crate) struct LayerConfig<C: CatalogSpec> {
    pub(crate) runtime: Arc<Runtime<C>>,
    pub(crate) request_ids: Arc<dyn RequestIdGenerator>,
    pub(crate) instance_uri: Arc<InstanceFactory>,
    pub(crate) request_id_header: HeaderName,
}

/// Cloneable Tower layer that installs Recourse request context.
pub struct RecourseLayer<C: CatalogSpec> {
    config: Arc<LayerConfig<C>>,
}

impl<C: CatalogSpec> RecourseLayer<C> {
    /// Starts strict adapter configuration around a validated catalog.
    pub fn builder(catalog: Catalog<C>) -> RecourseLayerBuilder<C> {
        RecourseLayerBuilder::new(catalog)
    }

    pub(crate) fn new(config: LayerConfig<C>) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<C: CatalogSpec> Clone for RecourseLayer<C> {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
        }
    }
}

impl<C: CatalogSpec> fmt::Debug for RecourseLayer<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecourseLayer")
            .finish_non_exhaustive()
    }
}

impl<S, C> Layer<S> for RecourseLayer<C>
where
    C: CatalogSpec,
{
    type Service = RecourseService<S, C>;

    fn layer(&self, inner: S) -> Self::Service {
        RecourseService {
            inner,
            config: Arc::clone(&self.config),
            readiness_failure: Arc::new(Mutex::new(None)),
        }
    }
}

/// Service produced by [`RecourseLayer`].
pub struct RecourseService<S, C: CatalogSpec> {
    inner: S,
    config: Arc<LayerConfig<C>>,
    readiness_failure: Arc<Mutex<Option<PrivateReport>>>,
}

impl<S: Clone, C: CatalogSpec> Clone for RecourseService<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: Arc::clone(&self.config),
            readiness_failure: Arc::clone(&self.readiness_failure),
        }
    }
}

impl<S, C> Service<Request<Body>> for RecourseService<S, C>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Error + Send + Sync + 'static,
    C: CatalogSpec,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.inner.poll_ready(context) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(error)) => {
                let report = private_report(error, "request_service_readiness");
                *lock_readiness_failure(&self.readiness_failure) = Some(report);
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let prepared = scope::prepare(&request, &self.config);
        let (problem_context, correlation_id) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                let response = preparation_failure(&request, &self.config, error);
                return Box::pin(async move { Ok(response) });
            }
        };
        let panic_context = problem_context.clone();
        let header = self.config.request_id_header.clone();
        if let Some(report) = lock_readiness_failure(&self.readiness_failure).take() {
            let mut response = problem_context.internal_fault(report).into_response();
            echo_request_id(&mut response, &header, &correlation_id);
            return Box::pin(async move { Ok(response) });
        }
        request.extensions_mut().insert(problem_context);
        let future = self.inner.call(request);
        Box::pin(async move {
            let outcome = std::panic::AssertUnwindSafe(future).catch_unwind().await;
            let mut response = match outcome {
                Ok(Ok(response)) => response,
                Ok(Err(error)) => {
                    let report = private_report(error, "request_service_error");
                    panic_context.internal_fault(report).into_response()
                }
                Err(payload) => {
                    let panic = RecoveredPanic::from_payload(payload.as_ref());
                    let report = private_report(panic, "request_service_panic");
                    panic_context.internal_fault(report).into_response()
                }
            };
            echo_request_id(&mut response, &header, &correlation_id);
            Ok(response)
        })
    }
}

fn lock_readiness_failure(
    state: &Mutex<Option<PrivateReport>>,
) -> std::sync::MutexGuard<'_, Option<PrivateReport>> {
    state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl<S, C: CatalogSpec> fmt::Debug for RecourseService<S, C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecourseService")
            .finish_non_exhaustive()
    }
}

fn preparation_failure<C: CatalogSpec>(
    request: &Request<Body>,
    config: &LayerConfig<C>,
    error: scope::ScopeError,
) -> Response {
    let Some(occurrence) = scope::emergency_occurrence() else {
        return config.runtime.empty_internal().into_response();
    };
    let correlation_id = occurrence.correlation_id().clone();
    let context = scope::event_context(request);
    let failure = config.runtime.fallback(
        &occurrence,
        &context,
        vec![private_report(error, "request_scope_preparation")],
    );
    let mut response = failure.into_response();
    echo_request_id(&mut response, &config.request_id_header, &correlation_id);
    response
}

fn echo_request_id(response: &mut Response, header: &HeaderName, id: &CorrelationId) {
    if let Ok(value) = HeaderValue::from_str(id.as_str()) {
        response.headers_mut().insert(header, value);
    }
}
