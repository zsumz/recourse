//! DSP-1004 reuse of an idempotency key with different job inputs.

use recourse::{
    catalog::CodeNumber,
    diagnostic::DiagnosticType,
    http::{Fixed, HttpProblemType},
};

use crate::{DispatchCatalog, IdempotencyConflictEvidence};

/// An idempotency key already identifies a job created from other inputs.
#[derive(Debug)]
pub enum IdempotencyConflict {}

impl DiagnosticType for IdempotencyConflict {
    type Catalog = DispatchCatalog;
    type Evidence = IdempotencyConflictEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1004);
    const TITLE: &'static str = "Idempotency key conflict";
    const DETAIL: &'static str =
        "This idempotency key already identifies a job created from different inputs.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Repeat the original request unchanged to read its result.",
        "Use a new idempotency key for different job inputs.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1004.md");
}

impl HttpProblemType for IdempotencyConflict {
    type Policy = Fixed<409>;
}
