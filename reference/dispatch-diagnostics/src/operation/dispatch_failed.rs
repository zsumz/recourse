//! DSP-1009 durable failure after a job was accepted.

use recourse::{
    catalog::CodeNumber, diagnostic::DiagnosticType, operation::OperationDiagnosticType,
};

use crate::{DispatchCatalog, DispatchFailedEvidence, DispatchImpact};

/// An accepted job could not complete its background dispatch.
#[derive(Debug)]
pub enum DispatchFailed {}

impl DiagnosticType for DispatchFailed {
    type Catalog = DispatchCatalog;
    type Evidence = DispatchFailedEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1009);
    const TITLE: &'static str = "Job dispatch failed";
    const DETAIL: &'static str = "The job was accepted but could not be dispatched.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Inspect the failed attempt.",
        "Retry after correcting the destination configuration.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1009.md");
}

impl OperationDiagnosticType for DispatchFailed {
    type Impact = DispatchImpact;
}
