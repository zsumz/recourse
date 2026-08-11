//! Exact catalog-lock fixture, bounded parser, and reservation lifecycle tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{
    AcceptanceMode, Catalog, CatalogArtifact, CatalogLock, CatalogSpec, Code, CodeNumber,
    LockState, Reservation, ReservationError, RetirementError,
};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Check the supplied job identifier.";
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}

macro_rules! replacement_diagnostic {
    ($name:ident, $number:literal) => {
        enum $name {}

        impl DiagnosticType for $name {
            type Catalog = DispatchCatalog;
            type Evidence = NoEvidence;
            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "Replacement-chain fixture.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Replacement-chain fixture.";
        }

        impl HttpProblemType for $name {
            type Policy = Fixed<400>;
        }
    };
}

replacement_diagnostic!(ReplacementOne, 2001);
replacement_diagnostic!(ReplacementTwo, 2002);
replacement_diagnostic!(ReplacementThree, 2003);

fn lock() -> CatalogLock {
    let artifact = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact();
    CatalogLock::from_artifact(&artifact)
}

fn replacement_artifact() -> CatalogArtifact {
    Catalog::<DispatchCatalog>::builder()
        .problem::<ReplacementOne>()
        .problem::<ReplacementTwo>()
        .problem::<ReplacementThree>()
        .build()
        .unwrap_or_else(|error| panic!("replacement catalog must build: {error}"))
        .artifact()
}

fn replacement_lock() -> CatalogLock {
    CatalogLock::from_artifact(&replacement_artifact())
}

fn code(number: u32) -> Code {
    format!("DSP-{number}")
        .parse()
        .unwrap_or_else(|error| panic!("fixture code must parse: {error}"))
}

fn assert_round_trip(lock: &CatalogLock) {
    let mut body = Vec::new();
    lock.write_pretty(&mut body)
        .unwrap_or_else(|error| panic!("lock must encode: {error}"));
    let decoded = CatalogLock::from_slice(&body)
        .unwrap_or_else(|error| panic!("written lock must parse: {error}"));
    assert_eq!(&decoded, lock);
}

#[test]
fn initial_lock_matches_exact_fixture_and_round_trips() {
    let lock = lock();
    let mut body = Vec::new();
    assert!(lock.write_pretty(&mut body).is_ok());

    assert_eq!(body, include_bytes!("lock_test_fixture.json"));
    assert_eq!(CatalogLock::from_slice(&body).ok(), Some(lock));
}

#[test]
fn reservations_never_reuse_history_or_scan_back_into_gaps() {
    let mut lock = lock();
    let gap = lock
        .reserve(Reservation::Exact(CodeNumber::new(1001)))
        .unwrap_or_else(|error| panic!("unused explicit number must reserve: {error}"));
    assert_eq!(gap.code().to_string(), "DSP-1001");
    assert_round_trip(&lock);
    let next = lock
        .reserve(Reservation::Next)
        .unwrap_or_else(|error| panic!("next number must reserve: {error}"));
    assert_eq!(next.code().to_string(), "DSP-1004");
    assert_round_trip(&lock);

    assert_eq!(
        lock.reserve(Reservation::Exact(CodeNumber::new(1003))),
        Err(ReservationError::AlreadyUsed {
            number: CodeNumber::new(1003)
        })
    );
}

#[test]
fn retirement_chains_round_trip_and_cycles_are_rejected_atomically() {
    let mut lock = replacement_lock();
    assert!(
        lock.retire(&code(2001), "Use the second diagnostic.", Some(code(2002)))
            .is_ok()
    );
    assert_round_trip(&lock);
    assert!(
        lock.retire(&code(2002), "Use the third diagnostic.", Some(code(2003)))
            .is_ok()
    );
    assert_round_trip(&lock);

    assert!(matches!(
        lock.retire(&code(2003), "Would cycle.", Some(code(2001))),
        Err(RetirementError::ReplacementCycle { .. })
    ));
    assert_eq!(lock.entries()[2].state(), LockState::Active);
    assert_round_trip(&lock);
}

#[test]
fn accepted_additions_round_trip() {
    let initial = Catalog::<DispatchCatalog>::builder()
        .problem::<ReplacementOne>()
        .build()
        .unwrap_or_else(|error| panic!("initial catalog must build: {error}"))
        .artifact();
    let mut lock = CatalogLock::from_artifact(&initial);

    assert!(
        lock.accept(&replacement_artifact(), AcceptanceMode::CompatibleOnly)
            .is_ok()
    );
    assert_round_trip(&lock);
}

#[test]
fn parser_rejects_a_replacement_cycle() {
    let mut lock = replacement_lock();
    assert!(
        lock.retire(&code(2001), "Use the second diagnostic.", Some(code(2002)))
            .is_ok()
    );
    assert!(
        lock.retire(&code(2002), "Use the third diagnostic.", Some(code(2003)))
            .is_ok()
    );
    let mut value = serde_json::to_value(lock)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    value["entries"][2]["state"] = serde_json::json!("retired");
    value["entries"][2]["reason"] = serde_json::json!("Cycle to the first diagnostic.");
    value["entries"][2]["replacement"] = serde_json::json!("DSP-2001");
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("cyclic fixture must encode: {error}"));

    let error = CatalogLock::from_slice(&body)
        .err()
        .unwrap_or_else(|| panic!("cyclic lock must be rejected"));
    assert!(error.to_string().contains("acyclic"));
}

#[test]
fn a_retired_entry_remains_a_valid_permanent_tombstone() {
    let mut value = serde_json::to_value(lock())
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    value["entries"][0]["state"] = serde_json::json!("retired");
    value["entries"][0]["reason"] = serde_json::json!("The resource no longer exists.");
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("retired fixture must encode: {error}"));
    let mut retired = CatalogLock::from_slice(&body)
        .unwrap_or_else(|error| panic!("retired lock must parse: {error}"));

    assert_eq!(retired.entries()[0].state(), LockState::Retired);
    assert!(matches!(
        retired.reserve(Reservation::Exact(CodeNumber::new(1003))),
        Err(ReservationError::AlreadyUsed { .. })
    ));
}

#[test]
fn parser_rejects_identity_drift_inside_a_reservation() {
    let mut lock = lock();
    assert!(lock.reserve(Reservation::Next).is_ok());
    let mut value = serde_json::to_value(lock)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    value["entries"][1]["type"] = serde_json::json!("https://attacker.invalid/DSP-1004");
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("mutated lock must encode: {error}"));

    assert!(CatalogLock::from_slice(&body).is_err());
}
