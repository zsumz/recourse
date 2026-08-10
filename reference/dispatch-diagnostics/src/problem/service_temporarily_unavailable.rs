//! DSP-1007 transient dependency or capacity outage.

use recourse::{
    catalog::CodeNumber,
    diagnostic::{DiagnosticType, NoEvidence},
    http::{HttpProblemType, RetryAfterPolicy},
};

use crate::DispatchCatalog;

/// Dispatch cannot serve the request until a transient condition clears.
#[derive(Debug)]
pub enum ServiceTemporarilyUnavailable {}

impl DiagnosticType for ServiceTemporarilyUnavailable {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1007);
    const TITLE: &'static str = "Service temporarily unavailable";
    const DETAIL: &'static str = "Dispatch cannot complete the request right now.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Wait for at least the delay in Retry-After before retrying.",
        "Use exponential backoff if the condition continues.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1007.md");
}

impl HttpProblemType for ServiceTemporarilyUnavailable {
    type Policy = RetryAfterPolicy<503>;
}
