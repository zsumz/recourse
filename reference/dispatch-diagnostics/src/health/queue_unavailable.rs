//! DSP-1010 queue unavailability across request and health surfaces.

use recourse::{
    catalog::CodeNumber,
    diagnostic::DiagnosticType,
    health::HealthFindingType,
    http::{HttpProblemType, RetryAfterPolicy},
};

use crate::{DispatchCatalog, QueueUnavailableEvidence};

/// Dispatch workers cannot currently reach the durable job queue.
#[derive(Debug)]
pub enum QueueUnavailable {}

impl DiagnosticType for QueueUnavailable {
    type Catalog = DispatchCatalog;
    type Evidence = QueueUnavailableEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1010);
    const TITLE: &'static str = "Job queue unavailable";
    const DETAIL: &'static str = "The worker cannot currently reach the job queue.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Check queue connectivity.",
        "Verify credentials and network policy.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1010.md");
}

impl HttpProblemType for QueueUnavailable {
    type Policy = RetryAfterPolicy<503>;
}

impl HealthFindingType for QueueUnavailable {}
