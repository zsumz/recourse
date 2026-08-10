//! Concrete Axum handler failure with deferred diagnostic reporting.

use std::{fmt, sync::Arc};

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use recourse::http::EncodedProblem;

use crate::observation::{ObservationHooks, PendingObservation};

/// Handler result using Recourse's single concrete Axum failure type.
///
/// A handler translates framework inputs and outputs and nothing else. An
/// expected problem is one expression; an unexpected fault is one expression
/// plus the private context an operator needs. Neither arm names a status, a
/// header, or a body.
///
/// ```
/// use axum::extract::{Path, State};
/// use recourse::{
///     catalog::{CatalogSpec, CodeNumber},
///     diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
///     fault::PrivateReport,
///     http::{Fixed, HttpProblemType},
/// };
/// use recourse_axum::{HandlerResult, ProblemContext};
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// # enum ServiceCatalog {}
/// # impl CatalogSpec for ServiceCatalog {
/// #     const NAME: &'static str = "example-service";
/// #     const PREFIX: &'static str = "EXM";
/// #     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
/// # }
/// #[derive(Debug, Serialize, JsonSchema)]
/// struct JobNotFoundEvidence {
///     job_id: String,
/// }
///
/// impl PublicEvidence for JobNotFoundEvidence {}
///
/// enum JobNotFound {}
///
/// impl DiagnosticType for JobNotFound {
///     type Catalog = ServiceCatalog;
///     type Evidence = JobNotFoundEvidence;
///
///     const NUMBER: CodeNumber = CodeNumber::new(1003);
///     const TITLE: &'static str = "Job not found";
///     const DETAIL: &'static str = "No job exists for the supplied identifier.";
///     const SUGGESTIONS: &'static [&'static str] = &["Check the job identifier."];
///     const DOCS: &'static str = "Create a job before requesting its status.";
/// }
///
/// impl HttpProblemType for JobNotFound {
///     type Policy = Fixed<404>;
/// }
/// # enum InternalError {}
/// # impl DiagnosticType for InternalError {
/// #     type Catalog = ServiceCatalog;
/// #     type Evidence = NoEvidence;
/// #     const NUMBER: CodeNumber = CodeNumber::new(1008);
/// #     const TITLE: &'static str = "Internal error";
/// #     const DETAIL: &'static str = "The request could not be completed.";
/// #     const SUGGESTIONS: &'static [&'static str] = &["Retry the request."];
/// #     const DOCS: &'static str = "Contact support with the request ID.";
/// # }
/// # impl HttpProblemType for InternalError {
/// #     type Policy = Fixed<500>;
/// # }
/// # #[derive(Clone)]
/// # struct AppState;
/// # impl AppState {
/// #     fn job(&self, _id: &str) -> Result<Option<String>, std::io::Error> {
/// #         Ok(None)
/// #     }
/// # }
/// type Problems = ProblemContext<ServiceCatalog>;
///
/// async fn get_job(
///     problems: Problems,
///     Path(job_id): Path<String>,
///     State(state): State<AppState>,
/// ) -> HandlerResult<String> {
///     state
///         .job(&job_id)
///         .map_err(|source| {
///             problems.fault::<InternalError>(
///                 NoEvidence,
///                 PrivateReport::new(source)
///                     .context("operation", "get_job")
///                     .context("job_id", job_id.clone()),
///             )
///         })?
///         .ok_or_else(|| problems.problem::<JobNotFound>(JobNotFoundEvidence { job_id }))
/// }
/// ```
pub type HandlerResult<T> = Result<T, HttpFailure>;

enum PreparedResponse {
    Problem(EncodedProblem),
    EmptyInternal,
}

/// Sanitized, encoded Problem ready to cross Axum's response boundary.
pub struct HttpFailure {
    inner: Box<HttpFailureInner>,
}

struct HttpFailureInner {
    response: PreparedResponse,
    observation: Option<PendingObservation>,
    hooks: Arc<ObservationHooks>,
}

impl HttpFailure {
    pub(crate) fn problem(
        response: EncodedProblem,
        observation: PendingObservation,
        hooks: Arc<ObservationHooks>,
    ) -> Self {
        Self {
            inner: Box::new(HttpFailureInner {
                response: PreparedResponse::Problem(response),
                observation: Some(observation),
                hooks,
            }),
        }
    }

    pub(crate) fn empty_internal(hooks: Arc<ObservationHooks>) -> Self {
        Self {
            inner: Box::new(HttpFailureInner {
                response: PreparedResponse::EmptyInternal,
                observation: None,
                hooks,
            }),
        }
    }

    fn status(&self) -> StatusCode {
        match &self.inner.response {
            PreparedResponse::Problem(problem) => problem.status(),
            PreparedResponse::EmptyInternal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Debug for HttpFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpFailure")
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}

impl IntoResponse for HttpFailure {
    fn into_response(self) -> Response {
        let mut inner = *self.inner;
        if let Some(observation) = inner.observation.take() {
            inner.hooks.emit(observation);
        }
        match inner.response {
            PreparedResponse::Problem(problem) => encoded_response(problem),
            PreparedResponse::EmptyInternal => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn encoded_response(problem: EncodedProblem) -> Response {
    let (status, headers, body) = problem.into_parts();
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}
