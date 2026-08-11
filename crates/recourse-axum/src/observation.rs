//! Deferred delivery of public metadata and private fault reports.

use std::{
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

use recourse::{
    fault::PrivateReport,
    observe::{FaultEvent, FaultReporter, HttpObserver, ProblemEvent},
};

pub(crate) struct ObservationHooks {
    observer: Arc<dyn HttpObserver>,
    reporter: Arc<dyn FaultReporter>,
}

impl ObservationHooks {
    pub(crate) fn new(observer: Arc<dyn HttpObserver>, reporter: Arc<dyn FaultReporter>) -> Self {
        Self { observer, reporter }
    }

    pub(crate) fn emit(&self, pending: PendingObservation) {
        match pending {
            PendingObservation::Problem(event) => {
                contain_hook(|| self.observer.on_problem(&event));
            }
            PendingObservation::Fault { event, reports } => {
                contain_hook(|| self.observer.on_fault(&event));
                for report in reports {
                    contain_hook(|| self.reporter.report_fault(&event, &report));
                }
            }
        }
    }
}

fn contain_hook(hook: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(hook));
}

impl fmt::Debug for ObservationHooks {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObservationHooks")
            .finish_non_exhaustive()
    }
}

pub(crate) enum PendingObservation {
    Problem(ProblemEvent),
    Fault {
        event: FaultEvent,
        reports: Vec<PrivateReport>,
    },
}
