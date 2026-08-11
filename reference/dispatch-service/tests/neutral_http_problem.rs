//! Complete HTTP Problems built and encoded without any web framework.
//!
//! Nothing here names Axum, Tower, or `recourse-axum`: the core API is the
//! normative HTTP API, and an adapter is only a small explicit translation
//! from `EncodedProblem` onto its own transport.

use std::time::Duration;

use dispatch_diagnostics::{
    JobNotFound, JobNotFoundEvidence, QueueUnavailable, QueueUnavailableEvidence, catalog,
};
use dispatch_model::JobId;
use http::{Response, StatusCode, header::CONTENT_TYPE};
use recourse::{
    client::{DecodeLimits, ProblemClassification, ReceivedProblem},
    http::{CorrelationId, EncodedProblem, ProblemOccurrence, RetryAfter},
};

const CORRELATION_ID: &str = "neutral-conformance-01";

fn occurrence() -> ProblemOccurrence {
    let correlation_id = CorrelationId::new(CORRELATION_ID)
        .unwrap_or_else(|error| panic!("correlation ID must be valid: {error}"));
    ProblemOccurrence::new(
        correlation_id,
        format!("https://api.dispatch.invalid/problem-occurrences/{CORRELATION_ID}"),
    )
    .unwrap_or_else(|error| panic!("occurrence must be valid: {error}"))
}

/// Performs the whole framework-neutral translation an adapter would perform.
fn respond(encoded: EncodedProblem) -> Response<Vec<u8>> {
    let (status, headers, body) = encoded.into_parts();
    let mut builder = Response::builder().status(status);
    if let Some(target) = builder.headers_mut() {
        target.extend(headers);
    }
    builder
        .body(body)
        .unwrap_or_else(|error| panic!("response must build: {error}"))
}

fn received(response: &Response<Vec<u8>>) -> ReceivedProblem {
    ReceivedProblem::from_slice(
        response.status(),
        response.headers(),
        response.body(),
        DecodeLimits::default(),
    )
    .unwrap_or_else(|error| panic!("canonical body must decode: {error}"))
}

#[test]
fn a_typed_problem_becomes_an_http_response_without_an_adapter() {
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let job_id = JobId::new("job_01K00000000000000000000000")
        .unwrap_or_else(|error| panic!("job ID must be valid: {error}"));
    let encoded = catalog
        .try_problem::<JobNotFound>(
            occurrence(),
            JobNotFoundEvidence {
                job_id: job_id.clone(),
            },
        )
        .unwrap_or_else(|error| panic!("registered problem must build: {error}"))
        .try_encode()
        .unwrap_or_else(|error| panic!("problem must encode: {error}"));

    let response = respond(encoded);

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.headers()[CONTENT_TYPE], "application/problem+json");
    let received = received(&response);
    assert!(received.protocol_issues().is_empty());
    assert!(matches!(
        catalog.classify(&received),
        ProblemClassification::Known(known)
            if known.diagnostic().code().to_string() == "DSP-1003" && known.is_conformant()
    ));
    let typed = received
        .try_as::<JobNotFound>()
        .unwrap_or_else(|error| panic!("known code must verify: {error}"))
        .unwrap_or_else(|| panic!("known code must match its declaration"));
    let evidence = typed
        .evidence()
        .unwrap_or_else(|error| panic!("typed evidence must decode: {error}"));
    assert_eq!(evidence.job_id, job_id);
}

#[test]
fn a_header_aware_policy_reaches_the_response_without_an_adapter() {
    let catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let encoded = catalog
        .try_problem_with::<QueueUnavailable>(
            occurrence(),
            QueueUnavailableEvidence {
                consecutive_failures: 3,
            },
            RetryAfter::after(Duration::from_secs(30)),
        )
        .unwrap_or_else(|error| panic!("registered problem must build: {error}"))
        .try_encode()
        .unwrap_or_else(|error| panic!("problem must encode: {error}"));

    let response = respond(encoded);

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()["retry-after"], "30");
    let received = received(&response);
    assert_eq!(
        received.body_status(),
        Some(StatusCode::SERVICE_UNAVAILABLE)
    );
    assert_eq!(
        received.code().map(ToString::to_string).as_deref(),
        Some("DSP-1010")
    );
    // The policy header and the canonical content type are the only headers
    // the framework-neutral boundary supplies.
    assert_eq!(response.headers().len(), 2);
}
