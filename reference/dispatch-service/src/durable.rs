//! Failed-attempt facts and the governed durable diagnostic they produce.

use dispatch_diagnostics::{
    DispatchCatalog, DispatchFailed, DispatchFailedEvidence, DispatchImpact,
};
use dispatch_model::{Job, JobId};
use recourse::{
    catalog::Catalog,
    operation::{OperationDiagnostic, OperationDiagnosticId},
};

use crate::{DispatchFault, DispatchService, JobIdGenerator};

const JOB_ID_PREFIX: &str = "job_";

/// Public facts reported after accepted work fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchFailure {
    job_id: JobId,
    attempt: u32,
    impact: DispatchImpact,
}

impl DispatchFailure {
    /// Describes one failed attempt and its caller-visible consequences.
    pub const fn new(job_id: JobId, attempt: u32, impact: DispatchImpact) -> Self {
        Self {
            job_id,
            attempt,
            impact,
        }
    }

    /// Accepted job whose dispatch failed.
    pub const fn job_id(&self) -> &JobId {
        &self.job_id
    }

    /// Attempt number recorded in public evidence.
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Public consequences of the failure.
    pub const fn impact(&self) -> &DispatchImpact {
        &self.impact
    }

    /// Derives the deterministic occurrence identity for this attempt.
    ///
    /// The identity is a pure function of the attempt so a repeated report
    /// resolves to the same durable record instead of a second one.
    pub fn try_diagnostic_id(&self) -> Result<OperationDiagnosticId, DispatchFault> {
        let job_id = self.job_id.as_str();
        let suffix = job_id.strip_prefix(JOB_ID_PREFIX).unwrap_or(job_id);
        OperationDiagnosticId::try_new(format!("dia_{suffix}-{}", self.attempt)).map_err(|error| {
            DispatchFault::new(error, "build_diagnostic_id").with("job_id", job_id)
        })
    }
}

/// Terminal job state paired with the diagnostic that explains it.
#[derive(Debug)]
pub struct FailedDispatch {
    job: Job,
    diagnostic: OperationDiagnostic<DispatchFailedEvidence, DispatchImpact>,
}

impl FailedDispatch {
    /// Job after its accepted-to-failed transition.
    pub const fn job(&self) -> &Job {
        &self.job
    }

    /// Governed durable diagnostic describing the failure.
    pub const fn diagnostic(&self) -> &OperationDiagnostic<DispatchFailedEvidence, DispatchImpact> {
        &self.diagnostic
    }
}

impl<G: JobIdGenerator> DispatchService<G> {
    /// Fails one accepted job and builds its governed durable diagnostic.
    ///
    /// An accepted request can succeed at the HTTP layer and fail later, so
    /// this outcome is a durable diagnostic rather than an HTTP Problem.
    pub fn try_fail_job(
        &self,
        catalog: &Catalog<DispatchCatalog>,
        failure: &DispatchFailure,
    ) -> Result<FailedDispatch, DispatchFault> {
        let diagnostic_id = failure.try_diagnostic_id()?;
        let job = self.mark_failed(failure.job_id()).map_err(|error| {
            DispatchFault::new(error, "fail_job").with("job_id", failure.job_id().as_str())
        })?;
        let diagnostic = catalog
            .try_operation::<DispatchFailed>(
                diagnostic_id,
                DispatchFailedEvidence {
                    job_id: failure.job_id().clone(),
                    attempt: failure.attempt(),
                },
                failure.impact().clone(),
            )
            .map_err(|error| {
                DispatchFault::new(error, "build_dispatch_failed")
                    .with("job_id", failure.job_id().as_str())
            })?;
        Ok(FailedDispatch { job, diagnostic })
    }
}
