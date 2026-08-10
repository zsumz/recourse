//! Typed `405` fallback preserving route-specific allowed methods.

use axum::{extract::MatchedPath, http::Method};
use dispatch_diagnostics::{DispatchCatalog, UnsupportedMethod};
use recourse::{diagnostic::NoEvidence, http::AllowedMethods};
use recourse_axum::{HandlerResult, ProblemContext};

const JOB_COLLECTION_METHODS: AllowedMethods = AllowedMethods::from_static(&[Method::POST]);
const JOB_RESOURCE_METHODS: AllowedMethods = AllowedMethods::from_static(&[Method::GET]);

pub(crate) async fn unsupported(
    problems: ProblemContext<DispatchCatalog>,
    path: MatchedPath,
) -> HandlerResult<()> {
    let allowed = match path.as_str() {
        "/jobs" => JOB_COLLECTION_METHODS,
        _ => JOB_RESOURCE_METHODS,
    };
    Err(problems.problem_with::<UnsupportedMethod>(NoEvidence, allowed))
}
