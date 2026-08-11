//! Problem encoding preserves the exact governed occurrence URI reference.

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
};

use super::{CorrelationId, Fixed, HttpProblemType, ProblemOccurrence};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "problem-uri-test";
    const PREFIX: &'static str = "PUT";
    const TYPE_BASE: &'static str = "https://problem.invalid/problems/";
}

enum Missing {}

impl DiagnosticType for Missing {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Missing";
    const DETAIL: &'static str = "The requested value is missing.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Exact occurrence identity test.";
}

impl HttpProblemType for Missing {
    type Policy = Fixed<404>;
}

#[test]
fn encoded_problem_keeps_rootless_paths_and_fragments_exactly() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<Missing>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    for instance in ["occurrence-123", "/occurrences/123#attempt-2"] {
        let id = CorrelationId::new("request-1")
            .unwrap_or_else(|error| panic!("test correlation ID must build: {error}"));
        let occurrence = ProblemOccurrence::new(id, instance)
            .unwrap_or_else(|error| panic!("test occurrence must build: {error}"));
        let problem = catalog
            .try_problem::<Missing>(occurrence, NoEvidence)
            .unwrap_or_else(|error| panic!("test Problem must build: {error}"));
        let encoded = problem
            .try_encode()
            .unwrap_or_else(|error| panic!("test Problem must encode: {error}"));
        let (_, _, body) = encoded.into_parts();
        let wire: serde_json::Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("test Problem body must decode: {error}"));

        assert_eq!(wire["instance"], instance);
    }
}
