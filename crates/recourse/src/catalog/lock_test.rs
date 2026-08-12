//! Exact catalog-lock fixture, bounded parser, and reservation lifecycle tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
    wire::WireLimits,
};

use super::{
    Catalog, CatalogLock, CatalogSpec, Code, CodeNumber, LockEntry, LockState, LockWriteError,
    MAX_CATALOG_LOCK_BYTES, MAX_CATALOG_LOCK_ENTRIES, Reservation, ReservationError,
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

fn lock() -> CatalogLock {
    let artifact = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact();
    CatalogLock::from_artifact(&artifact)
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

#[test]
fn parser_and_reservation_reject_an_exhausted_type_namespace() {
    let value = serde_json::json!({
        "schema_version": 1,
        "catalog": {
            "name": "capacity",
            "prefix": "DSP",
            "type_base": capacity_type_base("DSP")
        },
        "entries": []
    });
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("capacity fixture must encode: {error}"));
    assert!(CatalogLock::from_slice(&body).is_err());

    let mut unchecked = lock();
    unchecked.set_type_base_unchecked(capacity_type_base("DSP"));
    assert!(matches!(
        unchecked.reserve(Reservation::Exact(CodeNumber::new(1))),
        Err(ReservationError::TypeNamespaceTooLong { .. })
    ));
}

#[test]
fn bounded_writer_does_not_mutate_the_destination_on_overflow() {
    let mut unchecked = lock();
    unchecked.set_type_base_unchecked("x".repeat(MAX_CATALOG_LOCK_BYTES + 1));
    let mut destination = b"sentinel".to_vec();

    let error = unchecked.write_pretty(&mut destination).err();

    assert!(matches!(
        error,
        Some(LockWriteError::TooLarge { maximum }) if maximum == MAX_CATALOG_LOCK_BYTES
    ));
    assert_eq!(destination, b"sentinel");
}

#[test]
fn reservation_commits_only_a_parser_closed_candidate() {
    let entries = (1..=u32::try_from(MAX_CATALOG_LOCK_ENTRIES)
        .unwrap_or_else(|error| panic!("entry limit must fit u32: {error}")))
        .map(|number| {
            let number = CodeNumber::new(number);
            let code = Code::new("DSP", number)
                .unwrap_or_else(|error| panic!("fixture code must build: {error}"));
            let type_uri = format!("https://dispatch.invalid/problems/{code}");
            LockEntry::reserved(number, code, type_uri)
        })
        .collect::<Vec<_>>();
    let mut unchecked = lock();
    unchecked.replace_entries_unchecked(entries);
    let before = unchecked.clone();

    let error = unchecked.reserve(Reservation::Next).err();

    assert!(matches!(
        error,
        Some(ReservationError::InvalidGeneratedLock { .. })
    ));
    assert_eq!(unchecked, before);
}

#[test]
fn parsed_nested_schemas_are_canonicalized_before_writing() {
    let canonical = lock();
    let mut canonical_body = Vec::new();
    assert!(canonical.write_pretty(&mut canonical_body).is_ok());
    let mut value = serde_json::to_value(&canonical)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    let schema = value
        .pointer_mut("/entries/0/diagnostic/evidence_schema")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap_or_else(|| panic!("fixture must contain an evidence schema"));
    let mut reversed = std::mem::take(schema).into_iter().collect::<Vec<_>>();
    reversed.reverse();
    schema.extend(reversed);
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("reordered fixture must encode: {error}"));
    let parsed = CatalogLock::from_slice(&body)
        .unwrap_or_else(|error| panic!("reordered fixture must parse: {error}"));
    let mut parsed_body = Vec::new();
    assert!(parsed.write_pretty(&mut parsed_body).is_ok());

    assert_eq!(parsed_body, canonical_body);
}

fn capacity_type_base(prefix: &str) -> String {
    let one_digit_code_bytes = prefix.len() + 2;
    let base_bytes = WireLimits::DEFAULT_MAX_STRING_BYTES - one_digit_code_bytes;
    format!("https://{}/", "a".repeat(base_bytes - 8))
}
