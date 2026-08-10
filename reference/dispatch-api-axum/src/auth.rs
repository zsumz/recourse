//! Deliberately small bearer-auth translation for reference routes.

use axum::http::{HeaderMap, header::AUTHORIZATION};
use dispatch_diagnostics::{AuthenticationRequired, DispatchCatalog};
use recourse::{diagnostic::NoEvidence, http::BearerChallenge};
use recourse_axum::{HttpFailure, ProblemContext};

const DEMO_AUTHORIZATION: &str = "Bearer dispatch-demo";
const CHALLENGE: BearerChallenge = BearerChallenge::from_static("dispatch");

pub(crate) fn require(
    headers: &HeaderMap,
    problems: &ProblemContext<DispatchCatalog>,
) -> Result<(), HttpFailure> {
    let accepted = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == DEMO_AUTHORIZATION);
    if accepted {
        return Ok(());
    }
    Err(problems.problem_with::<AuthenticationRequired>(NoEvidence, CHALLENGE))
}
