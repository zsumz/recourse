//! DSP-1011 request representation not supported by the operation.

use recourse::{
    catalog::CodeNumber,
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use crate::DispatchCatalog;

/// Request declared a representation Dispatch does not accept.
#[derive(Debug)]
pub enum UnsupportedMediaType {}

impl DiagnosticType for UnsupportedMediaType {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1011);
    const TITLE: &'static str = "Unsupported media type";
    const DETAIL: &'static str = "This operation accepts application/json requests.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Encode the request body as JSON.",
        "Set Content-Type to application/json.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1011.md");
}

impl HttpProblemType for UnsupportedMediaType {
    type Policy = Fixed<415>;
}
