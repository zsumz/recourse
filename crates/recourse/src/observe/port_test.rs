//! Focused tests for separate metadata-only and private-report ports.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::fault::PrivateReport;

use super::{FaultEvent, FaultReporter, HttpObserver, ProblemEvent};

struct CountingObserver(AtomicUsize);

impl HttpObserver for CountingObserver {
    fn on_problem(&self, _event: &ProblemEvent) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    fn on_fault(&self, _event: &FaultEvent) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct CountingReporter(AtomicUsize);

impl FaultReporter for CountingReporter {
    fn report_fault(&self, _event: &FaultEvent, report: &PrivateReport) {
        if !report.to_string().is_empty() {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[test]
fn observation_is_optional_while_fault_reporting_is_implemented() {
    fn assert_observer<T: HttpObserver>() {}
    fn assert_reporter<T: FaultReporter>() {}

    assert_observer::<()>();
    assert_reporter::<CountingReporter>();
    let observer = CountingObserver(AtomicUsize::new(0));
    let reporter = CountingReporter(AtomicUsize::new(0));
    assert_eq!(observer.0.load(Ordering::Relaxed), 0);
    assert_eq!(reporter.0.load(Ordering::Relaxed), 0);
}
