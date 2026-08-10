//! DSP-1006 request method unavailable for the selected resource.

use recourse::{
    catalog::CodeNumber,
    diagnostic::{DiagnosticType, NoEvidence},
    http::{HttpProblemType, MethodNotAllowed},
};

use crate::DispatchCatalog;

/// Resource exists, but does not support the request method.
#[derive(Debug)]
pub enum UnsupportedMethod {}

impl DiagnosticType for UnsupportedMethod {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1006);
    const TITLE: &'static str = "Unsupported method";
    const DETAIL: &'static str = "This resource does not support the request method.";
    const SUGGESTIONS: &'static [&'static str] =
        &["Use one of the methods listed in the Allow response header."];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1006.md");
}

impl HttpProblemType for UnsupportedMethod {
    type Policy = MethodNotAllowed;
}
