//! Synchronous telemetry ports without a runtime or backend dependency.

use crate::fault::PrivateReport;

use super::{FaultEvent, ProblemEvent};

/// Metadata-only hook for expected Problems and unexpected Faults.
pub trait HttpObserver: Send + Sync + 'static {
    /// Observes an expected public Problem without its evidence values.
    fn on_problem(&self, _event: &ProblemEvent) {}

    /// Observes an unexpected Fault without its evidence or private report.
    fn on_fault(&self, _event: &FaultEvent) {}
}

/// Separate private-error reporting port for unexpected faults.
///
/// This port has no blanket no-op implementation. Dropping a private report
/// discards the only record of an unexpected failure, so an application states
/// that intent through its adapter configuration rather than by leaving the
/// port unset.
pub trait FaultReporter: Send + Sync + 'static {
    /// Reports one private error with the same bounded metadata sent to observers.
    fn report_fault(&self, event: &FaultEvent, report: &PrivateReport);
}

/// Metadata-only observation is optional, so `()` observes nothing.
impl HttpObserver for () {}
