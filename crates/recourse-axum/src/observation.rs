//! Deferred delivery of public metadata and private fault reports.

use std::{fmt, sync::Arc};

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
            PendingObservation::Problem(event) => self.observer.on_problem(&event),
            PendingObservation::Fault { event, reports } => {
                self.observer.on_fault(&event);
                for report in reports {
                    self.reporter.report_fault(&event, &report);
                }
            }
        }
    }
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
