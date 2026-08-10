//! Consuming builder for request lifecycle and fallback policy.

use std::{fmt, sync::Arc};

use http::HeaderName;
use recourse::{
    catalog::{Catalog, CatalogSpec},
    http::{CorrelationId, HttpPolicy, HttpProblemType},
    observe::{FaultReporter, HttpObserver},
};

use crate::{
    layer::{InstanceFactory, LayerConfig, RecourseLayer},
    observation::ObservationHooks,
    request_id::{RequestIdGenerator, UlidRequestIds},
    runtime::{InternalDefinition, Runtime},
};

use super::{LayerBuildError, faults::FaultChoice, internal::prepare_internal};

/// Consuming builder for request lifecycle and sanitized fallback policy.
pub struct RecourseLayerBuilder<C: CatalogSpec> {
    catalog: Arc<Catalog<C>>,
    internal: Option<Result<InternalDefinition<C>, LayerBuildError>>,
    request_ids: Arc<dyn RequestIdGenerator>,
    instance_uri: Arc<InstanceFactory>,
    observer: Arc<dyn HttpObserver>,
    faults: FaultChoice,
    request_id_header: HeaderName,
}

impl<C: CatalogSpec> RecourseLayerBuilder<C> {
    pub(crate) fn new(catalog: Catalog<C>) -> Self {
        Self {
            catalog: Arc::new(catalog),
            internal: None,
            request_ids: Arc::new(UlidRequestIds),
            instance_uri: Arc::new(default_instance_uri),
            observer: Arc::new(()),
            faults: FaultChoice::Unstated,
            request_id_header: HeaderName::from_static("x-request-id"),
        }
    }

    /// Selects and validates the static internal fallback diagnostic.
    #[must_use]
    pub fn internal<D>(mut self) -> Self
    where
        D: HttpProblemType<Catalog = C, Evidence = recourse::diagnostic::NoEvidence>,
        <D::Policy as HttpPolicy>::Input: Default,
    {
        self.internal = Some(prepare_internal::<C, D>(&self.catalog));
        self
    }

    /// Replaces the default ULID request-ID generator.
    #[must_use]
    pub fn request_ids(mut self, generator: impl RequestIdGenerator) -> Self {
        self.request_ids = Arc::new(generator);
        self
    }

    /// Replaces the RFC 9457 instance URI-reference factory.
    #[must_use]
    pub fn instance_uri(
        mut self,
        factory: impl Fn(&CorrelationId) -> String + Send + Sync + 'static,
    ) -> Self {
        self.instance_uri = Arc::new(factory);
        self
    }

    /// Replaces the metadata-only HTTP observer.
    #[must_use]
    pub fn observer(mut self, observer: impl HttpObserver) -> Self {
        self.observer = Arc::new(observer);
        self
    }

    /// Selects the private fault-reporting port.
    ///
    /// Calling this again replaces the earlier reporter: re-stating where
    /// private reports go refines one choice rather than reversing it. Pairing
    /// it with [`discard_faults`](Self::discard_faults) in either order does
    /// reverse it, and [`build`](Self::build) rejects that.
    #[must_use]
    pub fn fault_reporter(mut self, reporter: impl FaultReporter) -> Self {
        self.faults = self.faults.with_reporter(Arc::new(reporter));
        self
    }

    /// Deliberately drops every private report this layer produces.
    ///
    /// Faults still reach the configured observer and callers still receive the
    /// sanitized Problem; only the private source chain and its context are
    /// discarded. Choose this only when another boundary already records them.
    ///
    /// Calling this again changes nothing. Pairing it with
    /// [`fault_reporter`](Self::fault_reporter) in either order states opposite
    /// choices, and [`build`](Self::build) rejects that.
    #[must_use]
    pub fn discard_faults(mut self) -> Self {
        self.faults = self.faults.with_discard();
        self
    }

    /// Replaces the request-ID header name used for acceptance and echo.
    #[must_use]
    pub fn request_id_header(mut self, header: HeaderName) -> Self {
        self.request_id_header = header;
        self
    }

    /// Validates required configuration and constructs the Tower layer.
    ///
    /// Construction is fail-closed on fault reporting. A builder that names
    /// neither [`fault_reporter`](Self::fault_reporter) nor the deliberate
    /// [`discard_faults`](Self::discard_faults) opt-out fails with
    /// [`LayerBuildError::MissingFaultReporter`], so no configuration silently
    /// drops a private report. A builder that names both, in either order,
    /// fails with [`LayerBuildError::ContradictoryFaultReporting`] rather than
    /// letting the last call decide. Repeating one of them is not a
    /// contradiction: a later reporter replaces an earlier one, and a repeated
    /// discard changes nothing. Recourse ships no reporter: the port belongs to
    /// the application, as `FaultLog` does below.
    ///
    /// ```
    /// use axum::{Router, routing::get};
    /// use recourse::{
    ///     catalog::{Catalog, CatalogSpec, CodeNumber},
    ///     diagnostic::{DiagnosticType, NoEvidence},
    ///     fault::PrivateReport,
    ///     http::{Fixed, HttpProblemType},
    ///     observe::{FaultEvent, FaultReporter},
    /// };
    /// use recourse_axum::{RecourseLayer, UlidRequestIds};
    ///
    /// # enum ServiceCatalog {}
    /// # impl CatalogSpec for ServiceCatalog {
    /// #     const NAME: &'static str = "example-service";
    /// #     const PREFIX: &'static str = "EXM";
    /// #     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
    /// # }
    /// enum InternalError {}
    ///
    /// impl DiagnosticType for InternalError {
    ///     type Catalog = ServiceCatalog;
    ///     type Evidence = NoEvidence;
    ///
    ///     const NUMBER: CodeNumber = CodeNumber::new(1008);
    ///     const TITLE: &'static str = "Internal error";
    ///     const DETAIL: &'static str = "The request could not be completed.";
    ///     const SUGGESTIONS: &'static [&'static str] = &["Retry the request."];
    ///     const DOCS: &'static str = "Contact support with the request ID.";
    /// }
    ///
    /// impl HttpProblemType for InternalError {
    ///     type Policy = Fixed<500>;
    /// }
    ///
    /// /// Application-owned port: private reports never reach the caller.
    /// #[derive(Debug)]
    /// struct FaultLog;
    ///
    /// impl FaultReporter for FaultLog {
    ///     fn report_fault(&self, event: &FaultEvent, report: &PrivateReport) {
    ///         eprintln!("fault code={} {report}", event.problem_metadata().code());
    ///     }
    /// }
    ///
    /// # fn example() -> Result<Router, Box<dyn std::error::Error>> {
    /// let catalog = Catalog::<ServiceCatalog>::builder()
    ///     .problem::<InternalError>()
    ///     .build()?;
    /// let recourse = RecourseLayer::<ServiceCatalog>::builder(catalog)
    ///     .internal::<InternalError>()
    ///     .request_ids(UlidRequestIds)
    ///     .instance_uri(|correlation_id| {
    ///         format!("https://api.example.invalid/problem-occurrences/{correlation_id}")
    ///     })
    ///     .fault_reporter(FaultLog)
    ///     .build()?;
    ///
    /// # async fn list_jobs() -> &'static str { "[]" }
    /// Ok(Router::new().route("/jobs", get(list_jobs)).layer(recourse))
    /// # }
    /// # assert!(example().is_ok());
    /// ```
    pub fn build(self) -> Result<RecourseLayer<C>, LayerBuildError> {
        let internal = self.internal.ok_or(LayerBuildError::MissingInternal)??;
        let reporter = self.faults.into_reporter()?;
        let hooks = ObservationHooks::new(self.observer, reporter);
        let runtime = Arc::new(Runtime::new(self.catalog, internal, hooks));
        Ok(RecourseLayer::new(LayerConfig {
            runtime,
            request_ids: self.request_ids,
            instance_uri: self.instance_uri,
            request_id_header: self.request_id_header,
        }))
    }
}

impl<C: CatalogSpec> fmt::Debug for RecourseLayerBuilder<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RecourseLayerBuilder")
            .field("catalog", &self.catalog)
            .field("has_internal", &self.internal.is_some())
            .field("fault_reporting", &self.faults.stated())
            .field("request_id_header", &self.request_id_header)
            .finish_non_exhaustive()
    }
}

fn default_instance_uri(correlation_id: &CorrelationId) -> String {
    format!("/problem-occurrences/{correlation_id}")
}
