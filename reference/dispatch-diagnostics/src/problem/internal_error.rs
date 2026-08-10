//! DSP-1008 sanitized fallback for unexpected private application failures.

use recourse::{
    catalog::CodeNumber,
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use crate::DispatchCatalog;

/// Dispatch could not complete a request because of an unexpected fault.
#[derive(Debug)]
pub enum InternalError {}

impl DiagnosticType for InternalError {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1008);
    const TITLE: &'static str = "Internal error";
    const DETAIL: &'static str = "Dispatch could not complete the request.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Retry the request after a short delay.",
        "Contact support with the response correlation ID if the problem continues.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1008.md");
}

impl HttpProblemType for InternalError {
    type Policy = Fixed<500>;
}
