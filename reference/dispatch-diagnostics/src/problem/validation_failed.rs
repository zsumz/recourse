//! DSP-1002 syntactically valid instructions that violate input rules.

use recourse::{
    catalog::CodeNumber,
    diagnostic::DiagnosticType,
    http::{Fixed, HttpProblemType},
    validation::ValidationEvidence,
};

use crate::DispatchCatalog;

/// Request syntax was valid, but one or more supplied values were rejected.
#[derive(Debug)]
pub enum ValidationFailed {}

impl DiagnosticType for ValidationFailed {
    type Catalog = DispatchCatalog;
    type Evidence = ValidationEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1002);
    const TITLE: &'static str = "Validation failed";
    const DETAIL: &'static str = "The request contains values that Dispatch cannot accept.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Correct each violation listed in the evidence object.",
        "Send the corrected request again.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1002.md");
}

impl HttpProblemType for ValidationFailed {
    type Policy = Fixed<422>;
}
