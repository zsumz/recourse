//! Shared catalog, fallback encoder, and observation runtime.

use std::{error::Error, sync::Arc};

use http::StatusCode;
use recourse::{
    catalog::{Catalog, CatalogSpec, Code},
    diagnostic::PublicEvidence,
    fault::PrivateReport,
    http::{EncodedProblem, Problem, ProblemOccurrence},
    observe::{FaultEvent, HttpEventContext, ProblemEvent},
};

use crate::{
    failure::HttpFailure,
    observation::{ObservationHooks, PendingObservation},
};

pub(crate) type InternalEncoder<C> =
    dyn Fn(&Catalog<C>, ProblemOccurrence) -> Result<EncodedProblem, PrivateReport> + Send + Sync;

pub(crate) struct InternalDefinition<C: CatalogSpec> {
    pub(crate) code: Code,
    pub(crate) status: StatusCode,
    pub(crate) encode: Arc<InternalEncoder<C>>,
}

pub(crate) struct Runtime<C: CatalogSpec> {
    catalog: Arc<Catalog<C>>,
    internal: InternalDefinition<C>,
    hooks: Arc<ObservationHooks>,
}

impl<C: CatalogSpec> Runtime<C> {
    pub(crate) fn new(
        catalog: Arc<Catalog<C>>,
        internal: InternalDefinition<C>,
        hooks: ObservationHooks,
    ) -> Self {
        Self {
            catalog,
            internal,
            hooks: Arc::new(hooks),
        }
    }

    pub(crate) fn catalog(&self) -> &Catalog<C> {
        &self.catalog
    }

    pub(crate) fn empty_internal(&self) -> HttpFailure {
        HttpFailure::empty_internal(Arc::clone(&self.hooks))
    }

    pub(crate) fn expected<E: PublicEvidence>(
        &self,
        problem: &Problem<E>,
        context: &HttpEventContext,
    ) -> HttpFailure {
        let event = ProblemEvent::from_problem(problem, context, false);
        match problem.try_encode() {
            Ok(encoded) => HttpFailure::problem(
                encoded,
                PendingObservation::Problem(event),
                Arc::clone(&self.hooks),
            ),
            Err(error) => self.fallback(
                problem.occurrence(),
                context,
                vec![private_report(error, "public_problem_encoding")],
            ),
        }
    }

    pub(crate) fn fault<E: PublicEvidence>(
        &self,
        problem: &Problem<E>,
        report: PrivateReport,
        context: &HttpEventContext,
    ) -> HttpFailure {
        let event = FaultEvent::for_http(
            problem.code().clone(),
            problem.status(),
            problem.occurrence(),
            context,
            false,
        );
        match problem.try_encode() {
            Ok(encoded) => HttpFailure::problem(
                encoded,
                PendingObservation::Fault {
                    event,
                    reports: vec![report],
                },
                Arc::clone(&self.hooks),
            ),
            Err(error) => self.fallback(
                problem.occurrence(),
                context,
                vec![report, private_report(error, "fault_problem_encoding")],
            ),
        }
    }

    pub(crate) fn fallback(
        &self,
        occurrence: &ProblemOccurrence,
        context: &HttpEventContext,
        mut reports: Vec<PrivateReport>,
    ) -> HttpFailure {
        let event = FaultEvent::for_http(
            self.internal.code.clone(),
            self.internal.status,
            occurrence,
            context,
            true,
        );
        match (self.internal.encode)(&self.catalog, occurrence.clone()) {
            Ok(encoded) => HttpFailure::problem(
                encoded,
                PendingObservation::Fault { event, reports },
                Arc::clone(&self.hooks),
            ),
            Err(error) => {
                reports.push(error);
                self.hooks
                    .emit(PendingObservation::Fault { event, reports });
                HttpFailure::empty_internal(Arc::clone(&self.hooks))
            }
        }
    }
}

pub(crate) fn private_report(
    error: impl Error + Send + Sync + 'static,
    stage: &'static str,
) -> PrivateReport {
    PrivateReport::new(error).context("recourse_stage", stage)
}
