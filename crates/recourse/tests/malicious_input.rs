//! Exact hostile-input fixtures for tolerant decoding and terminal safety.

use http::{HeaderMap, StatusCode};
use recourse::client::{DecodeLimits, ReceivedProblem, escape_terminal};
use serde_json::Value;

fn decode_fixture(body: &[u8]) -> ReceivedProblem {
    ReceivedProblem::from_slice(
        StatusCode::INTERNAL_SERVER_ERROR,
        &HeaderMap::new(),
        body,
        DecodeLimits::default(),
    )
    .unwrap_or_else(|error| panic!("bounded hostile fixture must decode: {error}"))
}

fn assert_raw_preserved(body: &[u8], received: &ReceivedProblem) {
    let expected: Value = serde_json::from_slice(body)
        .unwrap_or_else(|error| panic!("fixture must be valid JSON: {error}"));
    assert_eq!(Value::Object(received.raw().clone()), expected);
}

#[test]
fn a_new_code_and_every_extension_survive_an_old_decoder() {
    let body = include_bytes!("fixtures/malicious-input/new-code.json");
    let received = decode_fixture(body);

    assert_eq!(
        received.code().map(ToString::to_string).as_deref(),
        Some("DSP-1999")
    );
    assert_raw_preserved(body, &received);
}

#[test]
fn wrong_typed_standard_members_remain_raw_data() {
    let body = include_bytes!("fixtures/malicious-input/wrong-typed-members.json");
    let received = decode_fixture(body);

    assert_eq!(received.type_uri(), None);
    assert_eq!(received.title(), None);
    assert_eq!(received.body_status(), None);
    assert_eq!(received.code(), None);
    assert_eq!(received.evidence(), None);
    assert_eq!(received.suggestions(), ["kept"]);
    assert_raw_preserved(body, &received);
}

#[test]
fn hostile_display_members_cannot_reach_terminal_controls() {
    let body = include_bytes!("fixtures/malicious-input/terminal-spoof.json");
    let received = decode_fixture(body);

    for text in [received.title(), received.detail()].into_iter().flatten() {
        let escaped = escape_terminal(text);
        assert!(!escaped.chars().any(char::is_control));
        assert!(!escaped.contains('\u{202e}'));
        assert!(!escaped.contains('\u{2066}'));
        assert!(!escaped.contains('\u{2069}'));
    }
    assert_raw_preserved(body, &received);
}
