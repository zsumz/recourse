//! Catalog-aware and typed HTTP conformance tests.

use http::{HeaderMap, HeaderValue, StatusCode, header::WWW_AUTHENTICATE};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{BearerUnauthorized, Fixed, HttpProblemType},
};

use super::{
    DecodeLimits, ProblemClassification, ProtocolIssue, ReceivedProblem, TypedProblemError,
};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "client-conformance";
    const PREFIX: &'static str = "CLI";
    const TYPE_BASE: &'static str = "https://client.invalid/problems/";
}

macro_rules! diagnostic {
    ($name:ident, $number:literal, $policy:ty) => {
        enum $name {}

        impl DiagnosticType for $name {
            type Catalog = TestCatalog;
            type Evidence = NoEvidence;

            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "HTTP conformance test diagnostic.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "HTTP conformance test diagnostic.";
        }

        impl HttpProblemType for $name {
            type Policy = $policy;
        }
    };
}

diagnostic!(NotFound, 1, Fixed<404>);
diagnostic!(AuthenticationRequired, 2, BearerUnauthorized);

fn catalog() -> Catalog<TestCatalog> {
    Catalog::<TestCatalog>::builder()
        .problem::<NotFound>()
        .problem::<AuthenticationRequired>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"))
}

fn received(body: &[u8], status: StatusCode, headers: &HeaderMap) -> ReceivedProblem {
    ReceivedProblem::from_slice(status, headers, body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("test Problem must decode: {error}"))
}

#[test]
fn known_problem_reports_catalog_status_mismatch_and_refuses_typed_access() {
    let problem = received(
        br#"{"type":"https://client.invalid/problems/CLI-1","code":"CLI-1"}"#,
        StatusCode::INTERNAL_SERVER_ERROR,
        &HeaderMap::new(),
    );
    let catalog = catalog();
    let ProblemClassification::Known(known) = catalog.classify(&problem) else {
        panic!("known code must classify");
    };

    assert!(!known.is_conformant());
    assert!(matches!(
        known.catalog_issues(),
        [ProtocolIssue::CatalogStatusMismatch { .. }]
    ));
    assert!(matches!(
        problem.try_as::<NotFound>(),
        Err(TypedProblemError::StatusMismatch { .. })
    ));
}

#[test]
fn required_headers_are_checked_by_catalog_and_typed_access() {
    let body = br#"{"type":"https://client.invalid/problems/CLI-2","code":"CLI-2"}"#;
    let missing = received(body, StatusCode::UNAUTHORIZED, &HeaderMap::new());
    let catalog = catalog();
    let ProblemClassification::Known(known) = catalog.classify(&missing) else {
        panic!("known code must classify");
    };

    assert!(matches!(
        known.catalog_issues(),
        [ProtocolIssue::MissingRequiredHeader { header }] if header == "www-authenticate"
    ));
    assert!(matches!(
        missing.try_as::<AuthenticationRequired>(),
        Err(TypedProblemError::MissingRequiredHeader {
            header: "www-authenticate"
        })
    ));

    let mut headers = HeaderMap::new();
    headers.insert(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    let conformant = received(body, StatusCode::UNAUTHORIZED, &headers);
    assert!(
        conformant
            .try_as::<AuthenticationRequired>()
            .is_ok_and(|typed| typed.is_some())
    );
}

#[test]
fn known_code_with_spoofed_type_is_never_conformant() {
    let problem = received(
        br#"{"type":"https://attacker.invalid/problem","code":"CLI-1"}"#,
        StatusCode::NOT_FOUND,
        &HeaderMap::new(),
    );
    let catalog = catalog();
    let ProblemClassification::Known(known) = catalog.classify(&problem) else {
        panic!("known code must classify");
    };

    assert!(!known.is_conformant());
    assert!(matches!(
        known.catalog_issues(),
        [ProtocolIssue::UnexpectedTypeForCode { .. }]
    ));
}
