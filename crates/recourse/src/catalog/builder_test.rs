//! Focused tests for explicit catalog construction and aggregated failures.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogIssue, CatalogSpec, CodeNumber};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct ConflictEvidence {
    original_job_id: String,
}

impl PublicEvidence for ConflictEvidence {}

enum IdempotencyConflict {}

impl DiagnosticType for IdempotencyConflict {
    type Catalog = DispatchCatalog;
    type Evidence = ConflictEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1004);
    const TITLE: &'static str = "Idempotency key conflict";
    const DETAIL: &'static str = "The key already identifies different inputs.";
    const SUGGESTIONS: &'static [&'static str] = &["Use a new idempotency key."];
    const DOCS: &'static str = "The key is permanently bound to its first request.";
}

impl HttpProblemType for IdempotencyConflict {
    type Policy = Fixed<409>;
}

enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &["Check the job identifier."];
    const DOCS: &'static str = "The job identifier is unknown to Dispatch.";
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}

#[test]
fn explicit_registration_builds_numeric_order() {
    let catalog = Catalog::<DispatchCatalog>::builder()
        .problem::<IdempotencyConflict>()
        .problem::<JobNotFound>()
        .build();

    assert!(catalog.is_ok());
    let Some(catalog) = catalog.ok() else {
        return;
    };
    let artifact = catalog.artifact();
    assert_eq!(artifact.name(), "dispatch");
    assert_eq!(artifact.diagnostics().len(), 2);
    assert_eq!(artifact.diagnostics()[0].number(), CodeNumber::new(1003));
    assert_eq!(artifact.diagnostics()[1].code().to_string(), "DSP-1004");
    assert_eq!(
        artifact.diagnostics()[1].type_uri(),
        "https://dispatch.invalid/problems/DSP-1004"
    );
}

#[test]
fn registering_one_marker_twice_is_idempotent() {
    let catalog = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .problem::<JobNotFound>()
        .build();

    assert_eq!(
        catalog
            .ok()
            .map(|value| value.artifact().diagnostics().len()),
        Some(1)
    );
}

enum DuplicateJobNotFound {}

impl DiagnosticType for DuplicateJobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Different meaning";
    const DETAIL: &'static str = "This marker must not reuse an existing number.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "A conflicting declaration.";
}

impl HttpProblemType for DuplicateJobNotFound {
    type Policy = Fixed<409>;
}

#[test]
fn different_markers_cannot_claim_one_number() {
    let error = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .problem::<DuplicateJobNotFound>()
        .build()
        .err();

    assert!(error.is_some_and(|value| {
        value.issues().contains(&CatalogIssue::DuplicateNumber {
            number: CodeNumber::new(1003),
        })
    }));
}

enum InvalidCatalog {}

impl CatalogSpec for InvalidCatalog {
    const NAME: &'static str = "Dispatch API";
    const PREFIX: &'static str = "d";
    const TYPE_BASE: &'static str = "problems";
}

#[derive(Debug, Serialize, JsonSchema)]
struct ScalarEvidence(String);

impl PublicEvidence for ScalarEvidence {}

enum BrokenDiagnostic {}

impl DiagnosticType for BrokenDiagnostic {
    type Catalog = InvalidCatalog;
    type Evidence = ScalarEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(9);
    const TITLE: &'static str = "";
    const DETAIL: &'static str = "";
    const SUGGESTIONS: &'static [&'static str] = &[""];
    const DOCS: &'static str = "";
}

impl HttpProblemType for BrokenDiagnostic {
    type Policy = Fixed<200>;
}

#[test]
fn build_reports_independent_definition_failures_together() {
    let error = Catalog::<InvalidCatalog>::builder()
        .problem::<BrokenDiagnostic>()
        .build()
        .err();

    let Some(error) = error else {
        return;
    };
    assert!(error.issues().len() >= 8);
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, CatalogIssue::InvalidName { .. }))
    );
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, CatalogIssue::UnsupportedEvidenceSchema { .. }))
    );
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, CatalogIssue::InvalidHttpStatus { .. }))
    );
}

enum NonHttpCatalog {}

impl CatalogSpec for NonHttpCatalog {
    const NAME: &'static str = "non-http-dispatch";
    const PREFIX: &'static str = "ALT";
    const TYPE_BASE: &'static str = "recourse://dispatch/problems/";
}

enum NonHttpDiagnostic {}

impl DiagnosticType for NonHttpDiagnostic {
    type Catalog = NonHttpCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Non-HTTP diagnostic";
    const DETAIL: &'static str = "An absolute non-HTTP URI remains valid protocol identity.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Non-HTTP absolute type bases are supported.";
}

impl HttpProblemType for NonHttpDiagnostic {
    type Policy = Fixed<500>;
}

#[test]
fn absolute_non_http_type_bases_are_valid() {
    let catalog = Catalog::<NonHttpCatalog>::builder()
        .problem::<NonHttpDiagnostic>()
        .build();

    assert_eq!(
        catalog
            .ok()
            .and_then(|value| value.artifact().diagnostics().first().cloned())
            .map(|diagnostic| diagnostic.type_uri().to_owned()),
        Some("recourse://dispatch/problems/ALT-1".to_owned())
    );
}
