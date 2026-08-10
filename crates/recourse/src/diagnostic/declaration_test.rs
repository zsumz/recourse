//! Focused tests for semantic diagnostic declarations.

use crate::catalog::{CatalogSpec, CodeNumber};

use super::{DiagnosticType, NoEvidence};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
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

#[test]
fn declaration_binds_identity_evidence_and_guidance() {
    assert_eq!(JobNotFound::NUMBER, CodeNumber::new(1003));
    assert_eq!(JobNotFound::TITLE, "Job not found");
    assert_eq!(JobNotFound::SUGGESTIONS, ["Check the job identifier."]);
    assert!(!JobNotFound::DETAIL.is_empty());
    assert!(!JobNotFound::DOCS.is_empty());
}
