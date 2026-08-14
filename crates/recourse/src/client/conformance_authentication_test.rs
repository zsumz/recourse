//! Receive-side conformance fixtures for governed authentication challenges.

use http::{HeaderMap, HeaderValue, StatusCode, header::WWW_AUTHENTICATE};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{BasicUnauthorized, BearerUnauthorized, HttpProblemType},
};

use super::{
    DecodeLimits, ProblemClassification, ProtocolIssue, ReceivedProblem, TypedProblemError,
};

enum AuthenticationCatalog {}

impl CatalogSpec for AuthenticationCatalog {
    const NAME: &'static str = "client-authentication-conformance";
    const PREFIX: &'static str = "AUT";
    const TYPE_BASE: &'static str = "https://client.invalid/problems/";
}

macro_rules! diagnostic {
    ($name:ident, $number:literal, $policy:ty) => {
        enum $name {}

        impl DiagnosticType for $name {
            type Catalog = AuthenticationCatalog;
            type Evidence = NoEvidence;

            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "Authentication conformance fixture.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Authentication conformance fixture.";
        }

        impl HttpProblemType for $name {
            type Policy = $policy;
        }
    };
}

diagnostic!(BasicRequired, 1, BasicUnauthorized);
diagnostic!(BearerRequired, 2, BearerUnauthorized);

fn catalog() -> Catalog<AuthenticationCatalog> {
    Catalog::<AuthenticationCatalog>::builder()
        .problem::<BasicRequired>()
        .problem::<BearerRequired>()
        .build()
        .unwrap_or_else(|error| panic!("authentication catalog must build: {error}"))
}

fn received(number: u32, headers: &HeaderMap) -> ReceivedProblem {
    let body = format!(
        "{{\"type\":\"https://client.invalid/problems/AUT-{number}\",\"code\":\"AUT-{number}\"}}"
    );
    ReceivedProblem::from_slice(
        StatusCode::UNAUTHORIZED,
        headers,
        body.as_bytes(),
        DecodeLimits::default(),
    )
    .unwrap_or_else(|error| panic!("authentication Problem must decode: {error}"))
}

fn headers(values: &[&'static str]) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for value in values {
        headers.append(WWW_AUTHENTICATE, HeaderValue::from_static(value));
    }
    headers
}

#[test]
fn basic_and_bearer_contracts_accept_their_case_insensitive_schemes() {
    for value in [
        "Basic realm=\"registry\"",
        "bAsIc realm=\"registry\"",
        "Basic realm=\"registry\", charset=\"UTF-8\"",
        "Basic REALM=\"registry\", CHARSET=utf-8",
        "Basic realm=\"registry\", extension=accepted",
        "Basic realm=\"registry, mirror\"",
    ] {
        assert_conformant::<BasicRequired>(1, &headers(&[value]));
    }
    for value in [
        "bEaReR realm=\"registry\"",
        "Bearer scope=\"repository:team/image:pull\"",
        "Bearer extension=accepted",
    ] {
        assert_conformant::<BearerRequired>(2, &headers(&[value]));
    }
}

#[test]
fn authentication_schemes_cannot_satisfy_each_others_contracts() {
    assert_mismatch::<BasicRequired>(1, &headers(&["Bearer realm=\"registry\""]));
    assert_mismatch::<BearerRequired>(2, &headers(&["Basic realm=\"registry\""]));
    assert_mismatch::<BasicRequired>(1, &headers(&["Unknown realm=\"registry\""]));
}

#[test]
fn malformed_or_incomplete_basic_challenges_are_mismatches() {
    for value in [
        "Basic",
        "Basic realm",
        "Basic realm=registry",
        "Basic realm=\"unterminated",
        "Basic realm=\"\"",
        "Basic realm=\"registry\", charset=\"latin-1\"",
        "Basic realm=\"registry\", charset=",
        "Basic realm=\"first\", realm=\"second\"",
        "Basic realm=\"first\", REALM=\"second\"",
        "Basic realm=\"registry\", charset=UTF-8, charset=UTF-8",
        "Basic realm=\"registry\", extension=one, EXTENSION=two",
    ] {
        assert_mismatch::<BasicRequired>(1, &headers(&[value]));
    }
}

#[test]
fn bearer_requires_one_unique_authentication_parameter() {
    for value in [
        "Bearer",
        "Bearer YWJjZA==",
        "Bearer realm=\"first\", realm=\"second\"",
        "Bearer scope=\"pull\", SCOPE=\"push\"",
        "Bearer extension=one, EXTENSION=two",
    ] {
        assert_mismatch::<BearerRequired>(2, &headers(&[value]));
    }
}

#[test]
fn one_valid_challenge_can_appear_among_multiple_fields_and_challenges() {
    assert_conformant::<BasicRequired>(
        1,
        &headers(&["Bearer realm=\"registry\"", "Basic realm=\"token\""]),
    );
    assert_conformant::<BasicRequired>(
        1,
        &headers(&["Digest realm=\"legacy, mirror\", Basic realm=\"token\""]),
    );
    assert_conformant::<BasicRequired>(
        1,
        &headers(&["Negotiate YWJjZA==, Basic realm=\"token\", Broken ???"]),
    );
    assert_conformant::<BasicRequired>(1, &headers(&[", , Basic realm=\"token\", ,"]));
    assert_conformant::<BearerRequired>(
        2,
        &headers(&["Basic realm=\"registry\", Bearer realm=\"token\""]),
    );
}

#[test]
fn absent_authentication_header_keeps_the_existing_missing_header_issue() {
    let received = received(1, &HeaderMap::new());
    let catalog = catalog();
    let ProblemClassification::Known(known) = catalog.classify(&received) else {
        panic!("known Basic code must classify");
    };

    assert!(matches!(
        known.catalog_issues(),
        [ProtocolIssue::MissingRequiredHeader { header }] if header == "www-authenticate"
    ));
    assert!(matches!(
        received.try_as::<BasicRequired>(),
        Err(TypedProblemError::MissingRequiredHeader {
            header: "www-authenticate"
        })
    ));
}

fn assert_conformant<D: HttpProblemType>(number: u32, headers: &HeaderMap) {
    let received = received(number, headers);
    let catalog = catalog();
    let ProblemClassification::Known(known) = catalog.classify(&received) else {
        panic!("known authentication code must classify");
    };

    assert!(
        known.is_conformant(),
        "issues: {:?}",
        known.catalog_issues()
    );
    assert!(received.try_as::<D>().is_ok_and(|typed| typed.is_some()));
}

fn assert_mismatch<D: HttpProblemType>(number: u32, headers: &HeaderMap) {
    let received = received(number, headers);
    let catalog = catalog();
    let ProblemClassification::Known(known) = catalog.classify(&received) else {
        panic!("known authentication code must classify");
    };

    assert!(matches!(
        known.catalog_issues(),
        [ProtocolIssue::RequiredHeaderMismatch { header, .. }] if header == "www-authenticate"
    ));
    assert!(matches!(
        received.try_as::<D>(),
        Err(TypedProblemError::RequiredHeaderMismatch {
            header: "www-authenticate",
            ..
        })
    ));
}
