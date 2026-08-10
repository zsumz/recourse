//! Catalog-typed request context for constructing concrete handler failures.

use std::{fmt, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use http::StatusCode;
use recourse::{
    catalog::CatalogSpec,
    diagnostic::PublicText,
    fault::PrivateReport,
    http::{HttpPolicy, HttpProblemType, ProblemOccurrence},
    observe::HttpEventContext,
};

use crate::{
    failure::HttpFailure,
    runtime::{Runtime, private_report},
};

/// Request-scoped constructor for catalog-governed Axum Problems and Faults.
pub struct ProblemContext<C: CatalogSpec> {
    runtime: Arc<Runtime<C>>,
    occurrence: ProblemOccurrence,
    event_context: HttpEventContext,
}

impl<C: CatalogSpec> ProblemContext<C> {
    pub(crate) fn new(
        runtime: Arc<Runtime<C>>,
        occurrence: ProblemOccurrence,
        event_context: HttpEventContext,
    ) -> Self {
        Self {
            runtime,
            occurrence,
            event_context,
        }
    }

    /// Constructs an expected Problem whose policy input has a default.
    pub fn problem<D>(&self, evidence: D::Evidence) -> HttpFailure
    where
        D: HttpProblemType<Catalog = C>,
        <D::Policy as HttpPolicy>::Input: Default,
    {
        match self
            .runtime
            .catalog()
            .try_problem::<D>(self.occurrence.clone(), evidence)
        {
            Ok(problem) => self.runtime.expected(&problem, &self.event_context),
            Err(error) => self.runtime.fallback(
                &self.occurrence,
                &self.event_context,
                vec![private_report(error, "public_problem_construction")],
            ),
        }
    }

    /// Constructs an expected Problem with explicit typed policy input.
    pub fn problem_with<D>(
        &self,
        evidence: D::Evidence,
        input: <D::Policy as HttpPolicy>::Input,
    ) -> HttpFailure
    where
        D: HttpProblemType<Catalog = C>,
    {
        match self
            .runtime
            .catalog()
            .try_problem_with::<D>(self.occurrence.clone(), evidence, input)
        {
            Ok(problem) => self.runtime.expected(&problem, &self.event_context),
            Err(error) => self.runtime.fallback(
                &self.occurrence,
                &self.event_context,
                vec![private_report(error, "policy_problem_construction")],
            ),
        }
    }

    /// Constructs an expected Problem with reviewed dynamic public detail.
    pub fn problem_with_detail<D>(&self, evidence: D::Evidence, detail: PublicText) -> HttpFailure
    where
        D: HttpProblemType<Catalog = C>,
        <D::Policy as HttpPolicy>::Input: Default,
    {
        match self.runtime.catalog().try_problem_with_detail::<D>(
            self.occurrence.clone(),
            evidence,
            detail,
        ) {
            Ok(problem) => self.runtime.expected(&problem, &self.event_context),
            Err(error) => self.runtime.fallback(
                &self.occurrence,
                &self.event_context,
                vec![private_report(error, "detailed_problem_construction")],
            ),
        }
    }

    /// Constructs an unexpected Fault with private operator detail.
    pub fn fault<D>(&self, evidence: D::Evidence, report: PrivateReport) -> HttpFailure
    where
        D: HttpProblemType<Catalog = C>,
        <D::Policy as HttpPolicy>::Input: Default,
    {
        match self
            .runtime
            .catalog()
            .try_problem::<D>(self.occurrence.clone(), evidence)
        {
            Ok(problem) => self.runtime.fault(&problem, report, &self.event_context),
            Err(error) => self.runtime.fallback(
                &self.occurrence,
                &self.event_context,
                vec![report, private_report(error, "fault_problem_construction")],
            ),
        }
    }

    /// Constructs a header-aware unexpected Fault.
    pub fn fault_with<D>(
        &self,
        evidence: D::Evidence,
        input: <D::Policy as HttpPolicy>::Input,
        report: PrivateReport,
    ) -> HttpFailure
    where
        D: HttpProblemType<Catalog = C>,
    {
        match self
            .runtime
            .catalog()
            .try_problem_with::<D>(self.occurrence.clone(), evidence, input)
        {
            Ok(problem) => self.runtime.fault(&problem, report, &self.event_context),
            Err(error) => self.runtime.fallback(
                &self.occurrence,
                &self.event_context,
                vec![report, private_report(error, "policy_fault_construction")],
            ),
        }
    }

    /// Identity shared by Problems, request headers, and operator events.
    pub const fn problem_occurrence(&self) -> &ProblemOccurrence {
        &self.occurrence
    }

    pub(crate) fn internal_fault(&self, report: PrivateReport) -> HttpFailure {
        self.runtime
            .fallback(&self.occurrence, &self.event_context, vec![report])
    }
}

impl<C: CatalogSpec> Clone for ProblemContext<C> {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            occurrence: self.occurrence.clone(),
            event_context: self.event_context.clone(),
        }
    }
}

impl<C: CatalogSpec> fmt::Debug for ProblemContext<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProblemContext")
            .field("occurrence", &self.occurrence)
            .field("event_context", &self.event_context)
            .finish_non_exhaustive()
    }
}

/// Empty internal rejection returned when the Recourse layer is absent.
#[derive(Debug, Clone, Copy)]
pub struct MissingProblemContext;

impl IntoResponse for MissingProblemContext {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl<S, C> FromRequestParts<S> for ProblemContext<C>
where
    S: Send + Sync,
    C: CatalogSpec,
{
    type Rejection = MissingProblemContext;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(MissingProblemContext)
    }
}
