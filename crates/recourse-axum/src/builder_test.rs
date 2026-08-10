//! Strict internal-diagnostic configuration tests.

use std::sync::{Arc, Mutex, PoisonError};

use recourse::{
    catalog::{Catalog, CatalogSpec, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    fault::PrivateReport,
    http::{Fixed, HttpProblemType},
    observe::{FaultEvent, FaultReporter},
};

use super::{LayerBuildError, RecourseLayer};

#[derive(Debug)]
enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "axum-builder-test";
    const PREFIX: &'static str = "ABT";
    const TYPE_BASE: &'static str = "https://axum.invalid/problems/";
}

#[derive(Debug)]
enum NotFound {}

impl DiagnosticType for NotFound {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Not found";
    const DETAIL: &'static str = "The resource does not exist.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Missing resource.";
}

impl HttpProblemType for NotFound {
    type Policy = Fixed<404>;
}

#[derive(Debug)]
enum Internal {}

impl DiagnosticType for Internal {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(2);
    const TITLE: &'static str = "Internal error";
    const DETAIL: &'static str = "The request could not be completed.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Unexpected failure.";
}

impl HttpProblemType for Internal {
    type Policy = Fixed<500>;
}

#[derive(Debug, Default)]
struct RecordingReporter(Arc<Mutex<usize>>);

impl FaultReporter for RecordingReporter {
    fn report_fault(&self, _event: &FaultEvent, _report: &PrivateReport) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) += 1;
    }
}

fn catalog() -> Catalog<TestCatalog> {
    Catalog::builder()
        .problem::<NotFound>()
        .problem::<Internal>()
        .build()
        .unwrap_or_else(|error| panic!("test catalog must build: {error}"))
}

#[test]
fn layer_requires_an_internal_diagnostic() {
    let result = RecourseLayer::builder(catalog()).build();
    let Err(error) = result else {
        panic!("missing internal diagnostic must fail closed");
    };

    assert!(matches!(error, LayerBuildError::MissingInternal));
}

#[test]
fn layer_rejects_a_non_server_internal_status() {
    let result = RecourseLayer::builder(catalog())
        .internal::<NotFound>()
        .discard_faults()
        .build();
    let Err(error) = result else {
        panic!("4xx fallback diagnostic must be rejected");
    };

    assert!(matches!(error, LayerBuildError::InternalStatus { .. }));
}

#[test]
fn layer_requires_an_explicit_fault_reporting_choice() {
    let result = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .build();
    let Err(error) = result else {
        panic!("an unstated fault-reporting choice must fail closed");
    };

    assert!(matches!(error, LayerBuildError::MissingFaultReporter));
}

#[test]
fn either_stated_fault_reporting_choice_builds_a_layer() {
    let reported = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .fault_reporter(RecordingReporter::default())
        .build();
    let discarded = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .discard_faults()
        .build();

    assert!(reported.is_ok());
    assert!(discarded.is_ok());
}

#[test]
fn stating_both_fault_reporting_choices_fails_in_either_order() {
    let reporter_first = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .fault_reporter(RecordingReporter::default())
        .discard_faults()
        .build();
    let discard_first = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .discard_faults()
        .fault_reporter(RecordingReporter::default())
        .build();

    assert!(matches!(
        reporter_first,
        Err(LayerBuildError::ContradictoryFaultReporting)
    ));
    assert!(matches!(
        discard_first,
        Err(LayerBuildError::ContradictoryFaultReporting)
    ));
}

#[test]
fn restating_one_fault_reporting_choice_is_not_a_contradiction() {
    let replaced = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .fault_reporter(RecordingReporter::default())
        .fault_reporter(RecordingReporter::default())
        .build();
    let discarded_twice = RecourseLayer::builder(catalog())
        .internal::<Internal>()
        .discard_faults()
        .discard_faults()
        .build();

    assert!(replaced.is_ok());
    assert!(discarded_twice.is_ok());
}
