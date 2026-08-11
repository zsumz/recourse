//! Tower request lifecycle integration and request-ID response echo.

use std::{
    convert::Infallible,
    error::Error,
    fmt,
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::Arc,
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
            readiness_failure: None,
        }
    }
}

/// Service produced by [`RecourseLayer`].
pub struct RecourseService<S, C: CatalogSpec> {
    inner: S,
    config: Arc<LayerConfig<C>>,
    readiness_failure: Option<PrivateReport>,
}

impl<S: Clone, C: CatalogSpec> Clone for RecourseService<S, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: Arc::clone(&self.config),
            readiness_failure: None,
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
        match catch_unwind(AssertUnwindSafe(|| self.inner.poll_ready(context))) {
            Err(payload) => {
                let panic = RecoveredPanic::from_payload(payload.as_ref());
                self.readiness_failure = Some(private_report(panic, "request_service_readiness"));
                Poll::Ready(Ok(()))
            }
            Ok(Poll::Ready(Ok(()))) => Poll::Ready(Ok(())),
            Ok(Poll::Ready(Err(error))) => {
                self.readiness_failure = Some(private_report(error, "request_service_readiness"));
                Poll::Ready(Ok(()))
            }
            Ok(Poll::Pending) => Poll::Pending,
        }
    }

    fn call(&mut self, mut request: Request<Body>) -> Self::Future {
        let readiness_failure = self.readiness_failure.take();
        let prepared = catch_unwind(AssertUnwindSafe(|| scope::prepare(&request, &self.config)));
        let (problem_context, correlation_id) = match prepared {
            Ok(Ok(prepared)) => prepared,
            Ok(Err(error)) => {
                let response =
                    preparation_failure(&request, &self.config, error, readiness_failure);
                return Box::pin(async move { Ok(response) });
            }
            Err(payload) => {
                let panic = RecoveredPanic::from_payload(payload.as_ref());
                let response =
                    preparation_failure(&request, &self.config, panic, readiness_failure);
                return Box::pin(async move { Ok(response) });
            }
        };
        let panic_context = problem_context.clone();
        let header = self.config.request_id_header.clone();
        if let Some(report) = readiness_failure {
            let mut response = problem_context.internal_fault(report).into_response();
            echo_request_id(&mut response, &header, &correlation_id);
            return Box::pin(async move { Ok(response) });
        }
        request.extensions_mut().insert(problem_context);
        let future = match catch_unwind(AssertUnwindSafe(|| self.inner.call(request))) {
            Ok(future) => future,
            Err(payload) => {
                let panic = RecoveredPanic::from_payload(payload.as_ref());
                let report = private_report(panic, "request_service_call");
                let mut response = panic_context.internal_fault(report).into_response();
                echo_request_id(&mut response, &header, &correlation_id);
                return Box::pin(async move { Ok(response) });
            }
        };
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
    error: impl Error + Send + Sync + 'static,
    readiness_failure: Option<PrivateReport>,
) -> Response {
    let Some(occurrence) = scope::emergency_occurrence() else {
        return config.runtime.empty_internal().into_response();
    };
    let correlation_id = occurrence.correlation_id().clone();
    let context = scope::event_context(request);
    let mut reports = readiness_failure.into_iter().collect::<Vec<_>>();
    reports.push(private_report(error, "request_scope_preparation"));
    let failure = config.runtime.fallback(&occurrence, &context, reports);
    let mut response = failure.into_response();
    echo_request_id(&mut response, &config.request_id_header, &correlation_id);
    response
}

fn echo_request_id(response: &mut Response, header: &HeaderName, id: &CorrelationId) {
    if let Ok(value) = HeaderValue::from_str(id.as_str()) {
        response.headers_mut().insert(header, value);
    }
}
