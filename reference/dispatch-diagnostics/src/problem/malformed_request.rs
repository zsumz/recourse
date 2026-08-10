//! DSP-1001 malformed JSON syntax or request framing.

use recourse::{
    catalog::CodeNumber,
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use crate::DispatchCatalog;

/// Request bytes could not be interpreted as the declared representation.
#[derive(Debug)]
pub enum MalformedRequest {}

impl DiagnosticType for MalformedRequest {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1001);
    const TITLE: &'static str = "Malformed request";
    const DETAIL: &'static str = "The request body is not valid JSON.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Check the request body for invalid or incomplete JSON syntax.",
        "Send the corrected request again.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1001.md");
}

impl HttpProblemType for MalformedRequest {
    type Policy = Fixed<400>;
}
