//! Metadata-shape tests proving events do not retain evidence values.

use http::{Method, StatusCode};

use crate::{
    catalog::{Catalog, CatalogSpec, Code, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{CorrelationId, Fixed, HttpProblemType, ProblemOccurrence},
};

use super::{EventSurface, FaultEvent, HttpEventContext, NormalizedRoute, ProblemEvent};

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "observe-test";
    const PREFIX: &'static str = "OBS";
    const TYPE_BASE: &'static str = "https://observe.invalid/problems/";
}

#[derive(Debug)]
enum Missing {}

impl DiagnosticType for Missing {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Missing";
    const DETAIL: &'static str = "The resource is missing.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Missing resource.";
}

impl HttpProblemType for Missing {
    type Policy = Fixed<404>;
}

#[test]
fn event_contains_only_bounded_protocol_metadata() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<Missing>()
        .build();
    let occurrence = CorrelationId::new("observe-request")
        .ok()
        .and_then(|id| ProblemOccurrence::new(id, "/problem-occurrences/observe-request").ok());
    let route = NormalizedRoute::new("/resources/{resource_id}").ok();
    let (Some(catalog), Some(occurrence), Some(route)) = (catalog.ok(), occurrence, route) else {
        return;
    };
    let problem = catalog.try_problem::<Missing>(occurrence, NoEvidence);
    let Some(problem) = problem.ok() else {
        return;
    };
    let context = HttpEventContext::new()
        .with_method(Method::GET)
        .with_route(route);
    let event = ProblemEvent::from_problem(&problem, &context, false);

    assert_eq!(event.code().to_string(), "OBS-1");
    assert_eq!(event.surface(), EventSurface::Http);
    assert_eq!(event.request_method(), Some(&Method::GET));
    assert!(!event.used_fallback_encoding());
}

#[test]
fn adapter_fallback_metadata_does_not_require_public_evidence() {
    let correlation_id = CorrelationId::new("fallback-request")
        .unwrap_or_else(|error| panic!("test request ID must be valid: {error}"));
    let occurrence = ProblemOccurrence::new(correlation_id, "/problems/fallback-request")
        .unwrap_or_else(|error| panic!("test occurrence must be valid: {error}"));
    let code = Code::new("TST", CodeNumber::new(9000))
        .unwrap_or_else(|error| panic!("test code must be canonical: {error}"));

    let event = FaultEvent::for_http(
        code,
        StatusCode::INTERNAL_SERVER_ERROR,
        &occurrence,
        &HttpEventContext::new().with_method(Method::POST),
        true,
    );

    let metadata = event.problem_metadata();
    assert_eq!(metadata.code().to_string(), "TST-9000");
    assert_eq!(metadata.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(metadata.used_fallback_encoding());
}
