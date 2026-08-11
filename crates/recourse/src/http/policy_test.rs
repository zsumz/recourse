//! Focused tests for sealed fixed HTTP policy metadata.

use crate::{
    catalog::{Catalog, CatalogIssue, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
};

use super::{Fixed, HttpPolicy, HttpProblemType};

#[test]
fn fixed_policy_has_one_status_and_no_required_headers() {
    type NotFound = Fixed<404>;

    assert_eq!(NotFound::STATUS, 404);
    assert_eq!(NotFound::NAME, "fixed");
    assert!(NotFound::REQUIRED_HEADERS.is_empty());
    assert!(NotFound::headers(()).is_ok_and(|headers| headers.is_empty()));
}

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "policy-test";
    const PREFIX: &'static str = "POL";
    const TYPE_BASE: &'static str = "https://policy.invalid/problems/";
}

enum HeaderlessUnauthorized {}

impl DiagnosticType for HeaderlessUnauthorized {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Headerless unauthorized";
    const DETAIL: &'static str = "A mandatory header is missing.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Fixed must not bypass status semantics.";
}

impl HttpProblemType for HeaderlessUnauthorized {
    type Policy = Fixed<401>;
}

#[test]
fn fixed_policy_cannot_bypass_status_mandated_headers() {
    let error = Catalog::<TestCatalog>::builder()
        .problem::<HeaderlessUnauthorized>()
        .build()
        .err();

    assert!(error.is_some_and(|error| {
        error
            .issues()
            .contains(&CatalogIssue::MissingMandatoryHeader {
                number: CodeNumber::new(1),
                status: 401,
                header: "www-authenticate",
            })
    }));
}
