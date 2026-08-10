//! Typed validation and erasure of the configured internal diagnostic.

use std::sync::Arc;

use recourse::{
    catalog::{Catalog, CatalogSpec},
    diagnostic::NoEvidence,
    http::{CorrelationId, HttpPolicy, HttpProblemType, ProblemOccurrence},
};

use crate::runtime::{InternalDefinition, private_report};

use super::LayerBuildError;

pub(super) fn prepare_internal<C, D>(
    catalog: &Catalog<C>,
) -> Result<InternalDefinition<C>, LayerBuildError>
where
    C: CatalogSpec,
    D: HttpProblemType<Catalog = C, Evidence = NoEvidence>,
    <D::Policy as HttpPolicy>::Input: Default,
{
    let correlation_id =
        CorrelationId::new("recourse-layer-probe").map_err(LayerBuildError::ValidationRequestId)?;
    let occurrence = ProblemOccurrence::new(correlation_id, "/problem-occurrences/layer-probe")
        .map_err(LayerBuildError::ValidationOccurrence)?;
    let problem = catalog
        .try_problem::<D>(occurrence, NoEvidence)
        .map_err(LayerBuildError::InternalProblem)?;
    if !problem.status().is_server_error() {
        return Err(LayerBuildError::InternalStatus {
            status: problem.status(),
        });
    }
    problem
        .try_encode()
        .map_err(LayerBuildError::InternalEncoding)?;

    Ok(InternalDefinition {
        code: problem.code().clone(),
        status: problem.status(),
        encode: Arc::new(|catalog, occurrence| {
            let problem = catalog
                .try_problem::<D>(occurrence, NoEvidence)
                .map_err(|error| private_report(error, "internal_problem_construction"))?;
            problem
                .try_encode()
                .map_err(|error| private_report(error, "internal_problem_encoding"))
        }),
    })
}
