//! Operator-only private context for internal Dispatch failures.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use recourse::fault::PrivateReport;

/// Internal Dispatch failure carrying its operator-only private report.
///
/// Recourse keeps public Problem values structurally separate from private
/// reports. The framework-neutral service owns the private half because it
/// knows the operation and identifiers involved; its caller decides how to
/// publish the sanitized public half and where to report this one.
#[derive(Debug)]
pub struct DispatchFault {
    report: PrivateReport,
}

impl DispatchFault {
    pub(crate) fn new<E>(source: E, operation: &'static str) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            report: PrivateReport::new(source).context("operation", operation),
        }
    }

    pub(crate) fn with(self, key: &'static str, value: impl Into<String>) -> Self {
        Self {
            report: self.report.context(key, value),
        }
    }

    /// Operator-only report describing this failure.
    pub const fn report(&self) -> &PrivateReport {
        &self.report
    }

    /// Moves the private report to a reporting boundary.
    pub fn into_report(self) -> PrivateReport {
        self.report
    }
}

impl Display for DispatchFault {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.report, formatter)
    }
}

impl Error for DispatchFault {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.report.source()
    }
}
