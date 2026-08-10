//! Leak-canary test for the structural public/private fault boundary.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use recourse_test::{catalog, occurrence};

use crate::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType, ProblemOccurrence},
};

use super::{Fault, PrivateReport};

const PRIVATE_CANARIES: [&str; 7] = [
    "postgres://diagnostic:private-password@db.internal/dispatch",
    "/srv/dispatch/private/tenant-42.json",
    "Bearer private-token-7cf4",
    "INSERT INTO jobs (tenant_secret) VALUES ('private-sql-canary')",
    "queue-01.private.cluster.local",
    "panic while decoding tenant credential",
    "x-api-key: private-request-header",
];

mod recourse_test {
    //! Local public Problem fixture for fault-boundary tests.

    use super::*;
    use crate::http::CorrelationId;

    #[derive(Debug)]
    pub(super) enum TestCatalog {}

    impl CatalogSpec for TestCatalog {
        const NAME: &'static str = "fault-test";
        const PREFIX: &'static str = "FLT";
        const TYPE_BASE: &'static str = "https://fault.invalid/problems/";
    }

    #[derive(Debug)]
    pub(super) enum InternalError {}

    impl DiagnosticType for InternalError {
        type Catalog = TestCatalog;
        type Evidence = NoEvidence;

        const NUMBER: CodeNumber = CodeNumber::new(1);
        const TITLE: &'static str = "Internal error";
        const DETAIL: &'static str = "The request could not be completed.";
        const SUGGESTIONS: &'static [&'static str] = &[];
        const DOCS: &'static str = "A sanitized fallback.";
    }

    impl HttpProblemType for InternalError {
        type Policy = Fixed<500>;
    }

    pub(super) fn catalog() -> Option<Catalog<TestCatalog>> {
        Catalog::<TestCatalog>::builder()
            .problem::<InternalError>()
            .build()
            .ok()
    }

    pub(super) fn occurrence() -> Option<ProblemOccurrence> {
        ProblemOccurrence::new(
            CorrelationId::new("fault-request").ok()?,
            "/problem-occurrences/fault-request",
        )
        .ok()
    }
}

#[derive(Debug)]
struct SecretError;

impl Display for SecretError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(PRIVATE_CANARIES[0])
    }
}

impl Error for SecretError {}

#[test]
fn fault_encoding_cannot_see_private_source_or_context() {
    let (Some(catalog), Some(occurrence)) = (catalog(), occurrence()) else {
        return;
    };
    let problem = catalog.try_problem::<recourse_test::InternalError>(occurrence, NoEvidence);
    let Some(problem) = problem.ok() else {
        return;
    };
    let report = PRIVATE_CANARIES[1..]
        .iter()
        .enumerate()
        .fold(PrivateReport::new(SecretError), |report, (index, value)| {
            report.context(format!("private_canary_{index}"), *value)
        });
    let fault = Fault::new(problem, report);
    let encoded = fault.try_encode();
    let Some(encoded) = encoded.ok() else {
        return;
    };
    let body = String::from_utf8_lossy(encoded.body());

    let private_report = fault.report().to_string();
    for canary in PRIVATE_CANARIES {
        assert!(!body.contains(canary), "public body leaked {canary}");
        assert!(
            private_report.contains(canary),
            "private report lost {canary}"
        );
    }
}
