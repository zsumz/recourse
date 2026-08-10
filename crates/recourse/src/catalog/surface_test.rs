//! Multi-surface catalog merging and impact-schema validation tests.

use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    diagnostic::{DiagnosticType, NoEvidence, PublicEvidence},
    health::HealthFindingType,
    http::{Fixed, HttpProblemType},
    operation::OperationDiagnosticType,
};

use super::{Catalog, CatalogIssue, CatalogSpec, CodeNumber};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "surface-test";
    const PREFIX: &'static str = "SUR";
    const TYPE_BASE: &'static str = "https://surface.invalid/problems/";
}

#[derive(Debug, Serialize, JsonSchema)]
struct QueueImpact {
    accepted_work_unchanged: bool,
}

impl PublicEvidence for QueueImpact {}

enum QueueUnavailable {}

impl DiagnosticType for QueueUnavailable {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(10);
    const TITLE: &'static str = "Queue unavailable";
    const DETAIL: &'static str = "The queue cannot accept work.";
    const SUGGESTIONS: &'static [&'static str] = &["Check queue connectivity."];
    const DOCS: &'static str = "Queue connectivity failed.";
}

impl HttpProblemType for QueueUnavailable {
    type Policy = Fixed<503>;
}

impl OperationDiagnosticType for QueueUnavailable {
    type Impact = QueueImpact;
}

impl HealthFindingType for QueueUnavailable {}

#[test]
fn one_identity_merges_three_explicit_surfaces() {
    let catalog = Catalog::<TestCatalog>::builder()
        .health::<QueueUnavailable>()
        .problem::<QueueUnavailable>()
        .operation::<QueueUnavailable>()
        .build()
        .unwrap_or_else(|error| panic!("multi-surface catalog must build: {error}"));
    let artifact = catalog.artifact();
    let Some(diagnostic) = artifact.diagnostics().first() else {
        panic!("registered diagnostic must exist");
    };

    assert_eq!(artifact.diagnostics().len(), 1);
    assert_eq!(diagnostic.http_status(), Some(503));
    assert_eq!(diagnostic.http_policy(), Some("fixed"));
    assert_eq!(diagnostic.required_headers(), Some(&[] as &[String]));
    assert!(diagnostic.supports_health());
    assert_eq!(
        diagnostic
            .impact_schema()
            .and_then(|schema| schema.pointer("/properties/accepted_work_unchanged/type"))
            .and_then(serde_json::Value::as_str),
        Some("boolean")
    );
}

#[derive(Debug, Serialize, JsonSchema)]
struct ScalarImpact(String);

impl PublicEvidence for ScalarImpact {}

enum InvalidImpact {}

impl DiagnosticType for InvalidImpact {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(11);
    const TITLE: &'static str = "Invalid impact";
    const DETAIL: &'static str = "The impact declaration is invalid.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Invalid test declaration.";
}

impl OperationDiagnosticType for InvalidImpact {
    type Impact = ScalarImpact;
}

#[test]
fn operation_impact_must_use_the_public_object_profile() {
    let error = Catalog::<TestCatalog>::builder()
        .operation::<InvalidImpact>()
        .build()
        .err();

    assert!(error.is_some_and(|error| {
        error.issues().iter().any(|issue| {
            matches!(
                issue,
                CatalogIssue::UnsupportedImpactSchema { number, .. }
                    if *number == CodeNumber::new(11)
            )
        })
    }));
}
