//! Retirement mutation, parsing, and replacement-chain invariants.

use crate::{
    catalog::{Catalog, CatalogArtifact, CatalogSpec, Code, CodeNumber},
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{AcceptanceMode, CatalogLock, LockState, MAX_RETIREMENT_REASON_CHARS, RetirementError};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

macro_rules! diagnostic {
    ($name:ident, $number:literal) => {
        enum $name {}

        impl DiagnosticType for $name {
            type Catalog = DispatchCatalog;
            type Evidence = NoEvidence;
            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = stringify!($name);
            const DETAIL: &'static str = "Retirement fixture.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Retirement fixture.";
        }

        impl HttpProblemType for $name {
            type Policy = Fixed<400>;
        }
    };
}

diagnostic!(ReplacementOne, 2001);
diagnostic!(ReplacementTwo, 2002);
diagnostic!(ReplacementThree, 2003);

fn artifact() -> CatalogArtifact {
    Catalog::<DispatchCatalog>::builder()
        .problem::<ReplacementOne>()
        .problem::<ReplacementTwo>()
        .problem::<ReplacementThree>()
        .build()
        .unwrap_or_else(|error| panic!("retirement catalog must build: {error}"))
        .artifact()
}

fn lock() -> CatalogLock {
    CatalogLock::from_artifact(&artifact())
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
    assert_eq!(CatalogLock::from_slice(&body).ok().as_ref(), Some(lock));
}

#[test]
fn retirement_chains_round_trip_and_cycles_are_rejected_atomically() {
    let mut lock = lock();
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
fn retirement_reasons_cannot_make_history_unreadable_or_unsafe() {
    for (reason, expected) in [
        (
            "x".repeat(MAX_RETIREMENT_REASON_CHARS + 1),
            RetirementError::ReasonTooLong {
                actual_chars: MAX_RETIREMENT_REASON_CHARS + 1,
                maximum: MAX_RETIREMENT_REASON_CHARS,
            },
        ),
        (
            "unsafe\nmarkdown".to_owned(),
            RetirementError::ReasonControlCharacter { character_index: 6 },
        ),
    ] {
        let mut lock = lock();
        assert_eq!(lock.retire(&code(2001), reason, None), Err(expected));
        assert_eq!(lock.entries()[0].state(), LockState::Active);
        assert_round_trip(&lock);
    }
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
        lock.accept(&artifact(), AcceptanceMode::CompatibleOnly)
            .is_ok()
    );
    assert_round_trip(&lock);
}

#[test]
fn parser_rejects_invalid_reasons_and_replacement_cycles() {
    let mut lock = lock();
    assert!(
        lock.retire(&code(2001), "Use the second diagnostic.", Some(code(2002)))
            .is_ok()
    );
    assert!(
        lock.retire(&code(2002), "Use the third diagnostic.", Some(code(2003)))
            .is_ok()
    );
    for reason in [
        "unsafe\nmarkdown".to_owned(),
        "x".repeat(MAX_RETIREMENT_REASON_CHARS + 1),
    ] {
        let mut value = serde_json::to_value(&lock)
            .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
        value["entries"][0]["reason"] = serde_json::json!(reason);
        let invalid_reason = serde_json::to_vec(&value)
            .unwrap_or_else(|error| panic!("reason fixture must encode: {error}"));
        assert!(CatalogLock::from_slice(&invalid_reason).is_err());
    }

    let mut value = serde_json::to_value(lock)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    value["entries"][2]["state"] = serde_json::json!("retired");
    value["entries"][2]["reason"] = serde_json::json!("Cycle to the first diagnostic.");
    value["entries"][2]["replacement"] = serde_json::json!("DSP-2001");
    let cycle = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("cyclic fixture must encode: {error}"));
    let error = CatalogLock::from_slice(&cycle)
        .err()
        .unwrap_or_else(|| panic!("cyclic lock must be rejected"));
    assert!(error.to_string().contains("acyclic"));
}
