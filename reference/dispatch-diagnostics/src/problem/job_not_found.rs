//! DSP-1003 lookup of an unknown Dispatch job identity.

use recourse::{
    catalog::CodeNumber,
    diagnostic::DiagnosticType,
    http::{Fixed, HttpProblemType},
};

use crate::{DispatchCatalog, JobNotFoundEvidence};

/// No Dispatch job exists for the supplied public identity.
#[derive(Debug)]
pub enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = JobNotFoundEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &[
        "Check the job identifier for transcription errors.",
        "Create a job before requesting its status.",
    ];
    const DOCS: &'static str = include_str!("../../catalog-text/DSP-1003.md");
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}
