//! DSP-1005 absent or unusable bearer authentication.

use recourse::{
    catalog::CodeNumber,
    diagnostic::{DiagnosticType, NoEvidence},
    http::{BearerUnauthorized, HttpProblemType},
};

use crate::DispatchCatalog;

/// Request did not carry credentials accepted by Dispatch.
#[derive(Debug)]
pub enum AuthenticationRequired {}

impl DiagnosticType for AuthenticationRequired {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1005);
    const TITLE: &'static str = "Authentication required";
    const DETAIL: &'static str = "A valid bearer token is required for this request.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Send a bearer token issued for the Dispatch API.",
        "Obtain a new token if the current token has expired.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1005.md");
}

impl HttpProblemType for AuthenticationRequired {
    type Policy = BearerUnauthorized;
}
