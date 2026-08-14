//! End-to-end typed policy-input tests through canonical Problem encoding.

use std::time::Duration;

use http::{
    Method, StatusCode,
    header::{ALLOW, RETRY_AFTER, WWW_AUTHENTICATE},
};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
};

use super::{
    AllowedMethods, BasicChallenge, BasicUnauthorized, BearerChallenge, BearerUnauthorized,
    CorrelationId, HttpProblemType, MethodNotAllowed, ProblemOccurrence, RetryAfter,
    RetryAfterPolicy,
};

#[derive(Debug)]
enum PolicyCatalog {}

impl CatalogSpec for PolicyCatalog {
    const NAME: &'static str = "policy-test";
    const PREFIX: &'static str = "POL";
    const TYPE_BASE: &'static str = "https://policy.invalid/problems/";
}

macro_rules! diagnostic {
    ($name:ident, $number:literal, $policy:ty) => {
        #[derive(Debug)]
        enum $name {}

        impl DiagnosticType for $name {
            type Catalog = PolicyCatalog;
            type Evidence = NoEvidence;

            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "A typed policy test diagnostic.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Policy test documentation.";
        }

        impl HttpProblemType for $name {
            type Policy = $policy;
        }
    };
}

diagnostic!(AuthenticationRequired, 1, BearerUnauthorized);
diagnostic!(UnsupportedMethod, 2, MethodNotAllowed);
diagnostic!(Unavailable, 3, RetryAfterPolicy<503>);
diagnostic!(RegistryAuthenticationRequired, 4, BasicUnauthorized);

fn catalog() -> Option<Catalog<PolicyCatalog>> {
    Catalog::<PolicyCatalog>::builder()
        .problem::<AuthenticationRequired>()
        .problem::<UnsupportedMethod>()
        .problem::<Unavailable>()
        .problem::<RegistryAuthenticationRequired>()
        .build()
        .ok()
}

fn occurrence() -> Option<ProblemOccurrence> {
    ProblemOccurrence::new(
        CorrelationId::new("policy-request").ok()?,
        "/problem-occurrences/policy-request",
    )
    .ok()
}

#[test]
fn typed_policy_inputs_reach_final_response_headers() {
    let (Some(catalog), Some(occurrence)) = (catalog(), occurrence()) else {
        return;
    };
    assert!(catalog.artifact().diagnostics().iter().any(|diagnostic| {
        diagnostic.number() == CodeNumber::new(4)
            && diagnostic.http_policy() == Some("basic_unauthorized")
            && diagnostic
                .required_headers()
                .is_some_and(|headers| headers == ["www-authenticate"])
    }));
    let Some(challenge) = BearerChallenge::new("dispatch").ok() else {
        return;
    };
    let Some(methods) = AllowedMethods::new([Method::GET, Method::HEAD]).ok() else {
        return;
    };
    let Some(registry_challenge) = BasicChallenge::new("registry").ok() else {
        return;
    };
    let unauthorized = catalog
        .try_problem_with::<AuthenticationRequired>(occurrence.clone(), NoEvidence, challenge)
        .ok()
        .and_then(|problem| problem.try_encode().ok());
    let method = catalog
        .try_problem_with::<UnsupportedMethod>(occurrence.clone(), NoEvidence, methods)
        .ok()
        .and_then(|problem| problem.try_encode().ok());
    let registry = catalog
        .try_problem_with::<RegistryAuthenticationRequired>(
            occurrence.clone(),
            NoEvidence,
            registry_challenge,
        )
        .ok()
        .and_then(|problem| problem.try_encode().ok());
    let retry = catalog
        .try_problem_with::<Unavailable>(
            occurrence,
            NoEvidence,
            RetryAfter::after(Duration::from_secs(30)),
        )
        .ok()
        .and_then(|problem| problem.try_encode().ok());

    assert!(unauthorized.is_some_and(|value| {
        value.status() == StatusCode::UNAUTHORIZED && value.headers().contains_key(WWW_AUTHENTICATE)
    }));
    assert!(method.is_some_and(|value| value.headers().contains_key(ALLOW)));
    assert!(registry.is_some_and(|value| {
        value.status() == StatusCode::UNAUTHORIZED
            && value
                .headers()
                .get(WWW_AUTHENTICATE)
                .is_some_and(|header| header == "Basic realm=\"registry\"")
    }));
    assert!(retry.is_some_and(|value| value.headers().get(RETRY_AFTER).is_some_and(|v| v == "30")));
}
