//! Tolerant unknown-code, classification, issue, and typed-access tests.

use http::{HeaderMap, StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, PublicEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{
    DecodeLimits, ProblemClassification, ProtocolIssue, ReceivedProblem, TypedProblemError,
};

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "received-test";
    const PREFIX: &'static str = "RCV";
    const TYPE_BASE: &'static str = "https://client.invalid/problems/";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
struct KnownEvidence {
    resource: String,
}

impl PublicEvidence for KnownEvidence {}

#[derive(Debug)]
enum Known {}

impl DiagnosticType for Known {
    type Catalog = TestCatalog;
    type Evidence = KnownEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Known";
    const DETAIL: &'static str = "Known diagnostic.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Known diagnostic.";
}

impl HttpProblemType for Known {
    type Policy = Fixed<404>;
}

fn received(body: &[u8], status: StatusCode) -> ReceivedProblem {
    ReceivedProblem::from_slice(status, &HeaderMap::new(), body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("test Problem must decode: {error}"))
}

#[test]
fn unknown_and_wrong_typed_members_remain_in_the_raw_object() {
    let problem = received(
        br#"{"type":"https://new.invalid/NEW-9000","title":42,"status":599,"code":"NEW-9000","evidence":{},"vendor":{"kept":true}}"#,
        StatusCode::BAD_GATEWAY,
    );

    assert_eq!(problem.transport_status(), StatusCode::BAD_GATEWAY);
    assert_eq!(problem.title(), None);
    assert_eq!(problem.raw()["title"], 42);
    assert_eq!(problem.raw()["vendor"]["kept"], true);
    assert!(matches!(
        problem.protocol_issues(),
        [
            ProtocolIssue::TransportStatusMismatch { .. },
            ProtocolIssue::InvalidMemberType {
                member: "title",
                expected: "string"
            }
        ]
    ));
}

#[test]
fn old_catalog_classifies_new_code_without_rejecting_it() {
    let catalog = Catalog::<TestCatalog>::builder()
        .problem::<Known>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"));
    let unknown = received(
        br#"{"type":"https://new.invalid/NEW-9000","status":500,"code":"NEW-9000","evidence":{},"future":7}"#,
        StatusCode::INTERNAL_SERVER_ERROR,
    );

    assert!(matches!(
        catalog.classify(&unknown),
        ProblemClassification::Unknown
    ));
    assert_eq!(unknown.raw()["future"], 7);
}

#[test]
fn typed_access_verifies_identity_and_preserves_extra_evidence() {
    let problem = received(
        br#"{"type":"https://client.invalid/problems/RCV-1","status":404,"code":"RCV-1","evidence":{"resource":"job","future":true}}"#,
        StatusCode::NOT_FOUND,
    );
    let typed = problem
        .try_as::<Known>()
        .unwrap_or_else(|error| panic!("matching type must verify: {error}"))
        .unwrap_or_else(|| panic!("matching code must produce a typed view"));

    assert_eq!(
        typed.evidence().ok(),
        Some(KnownEvidence {
            resource: "job".to_owned()
        })
    );
    assert_eq!(problem.raw()["evidence"]["future"], true);
}

#[test]
fn typed_conformance_requires_decodable_evidence() {
    let wrong_shape = received(
        br#"{"type":"https://client.invalid/problems/RCV-1","status":404,"code":"RCV-1","evidence":{"resource":7}}"#,
        StatusCode::NOT_FOUND,
    );
    let typed = wrong_shape
        .try_as::<Known>()
        .unwrap_or_else(|error| panic!("matching type must verify: {error}"))
        .unwrap_or_else(|| panic!("matching code must produce a typed view"));

    assert!(wrong_shape.protocol_issues().is_empty());
    assert!(matches!(
        typed.evidence(),
        Err(TypedProblemError::Evidence(_))
    ));
    assert!(!typed.is_conformant());
}

#[test]
fn typed_access_surfaces_spoofed_type_and_ignores_other_codes() {
    let spoofed = received(
        br#"{"type":"https://attacker.invalid/problem","code":"RCV-1","evidence":{}}"#,
        StatusCode::NOT_FOUND,
    );
    assert!(matches!(
        spoofed.try_as::<Known>(),
        Err(TypedProblemError::TypeMismatch { .. })
    ));
    let other = received(
        br#"{"type":"https://client.invalid/problems/RCV-2","code":"RCV-2","evidence":{}}"#,
        StatusCode::NOT_FOUND,
    );
    assert!(matches!(other.try_as::<Known>(), Ok(None)));
}

#[test]
fn malformed_standard_identity_is_a_nonfatal_issue() {
    let problem = received(
        br#"{"code":"not canonical","status":99,"evidence":[]}"#,
        StatusCode::BAD_REQUEST,
    );

    assert_eq!(problem.code(), None);
    assert_eq!(problem.evidence(), None);
    assert_eq!(
        problem.protocol_issues(),
        [
            ProtocolIssue::MalformedCode,
            ProtocolIssue::InvalidBodyStatus,
            ProtocolIssue::InvalidMemberType {
                member: "evidence",
                expected: "object"
            }
        ]
    );
}

#[test]
fn remote_problem_rejects_duplicate_members_before_classification() {
    let error = ReceivedProblem::from_slice(
        StatusCode::NOT_FOUND,
        &HeaderMap::new(),
        br#"{"code":"RCV-1","evidence":{"id":1,"id":2}}"#,
        DecodeLimits::default(),
    )
    .err()
    .unwrap_or_else(|| panic!("duplicate member must be rejected"));

    assert!(matches!(error, super::DecodeError::MalformedJson(_)));
}
