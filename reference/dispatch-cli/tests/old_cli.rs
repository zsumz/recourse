//! Old-client compatibility and hostile terminal-input fixtures.

use dispatch_cli::render_problem;
use dispatch_diagnostics::catalog;
use http::{HeaderMap, StatusCode};
use recourse::client::{DecodeLimits, ReceivedProblem};

fn receive(body: &[u8], status: StatusCode) -> ReceivedProblem {
    ReceivedProblem::from_slice(status, &HeaderMap::new(), body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("bounded fixture must decode: {error}"))
}

#[test]
fn old_cli_preserves_a_new_server_code_and_all_raw_data() {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let received = receive(
        include_bytes!("../../../crates/recourse/tests/fixtures/malicious-input/new-code.json"),
        StatusCode::BAD_GATEWAY,
    );
    let rendered = render_problem(&old_catalog, &received)
        .unwrap_or_else(|error| panic!("raw fixture must render: {error}"));

    assert!(rendered.starts_with("Unknown diagnostic DSP-1999\nHTTP 502\n"));
    assert!(rendered.contains("\\u{1b}[31mFuture failure\\u{1b}[0m"));
    assert!(rendered.contains("first\\n\\u{202e}second"));
    assert!(rendered.contains(r#""future":{"nested":[1,2,3]}"#));
    assert!(rendered.contains(r#""vendor":"kept""#));
    assert!(!rendered.contains('\u{1b}'));
    assert!(!rendered.contains('\u{202e}'));
}

#[test]
fn known_code_uses_local_identity_but_retains_remote_extensions() {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let received = receive(
        br#"{"type":"https://dispatch.invalid/problems/DSP-1003","title":"spoofed","detail":"Missing job\u001b[2J","status":404,"code":"DSP-1003","evidence":{"job_id":"01JTEST","extra":true}}"#,
        StatusCode::NOT_FOUND,
    );
    let rendered = render_problem(&old_catalog, &received)
        .unwrap_or_else(|error| panic!("known fixture must render: {error}"));

    assert!(rendered.starts_with("DSP-1003 — Job not found\nHTTP 404\n"));
    assert!(rendered.contains("Detail: Missing job\\u{1b}[2J"));
    assert!(!rendered.contains("\nTitle: spoofed\n"));
    assert!(rendered.contains(r#""title":"spoofed""#));
    assert!(rendered.contains(r#""extra":true"#));
    // The code and type match, so typed access runs and reports the evidence
    // that does not fit this client's declaration instead of failing.
    assert!(rendered.contains(
        "Protocol issue: decode typed evidence: job identifier must begin with 'job_'\n"
    ));
    assert!(!rendered.contains("\nJob: "));
    assert!(!rendered.contains('\u{1b}'));
}

#[test]
fn a_known_code_with_valid_evidence_is_read_through_its_declaration() {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let received = receive(
        br#"{"type":"https://dispatch.invalid/problems/DSP-1003","status":404,"code":"DSP-1003","evidence":{"job_id":"job_01K00000000000000000000000","added_later":7}}"#,
        StatusCode::NOT_FOUND,
    );
    let rendered = render_problem(&old_catalog, &received)
        .unwrap_or_else(|error| panic!("typed fixture must render: {error}"));

    assert!(rendered.contains("Job: job_01K00000000000000000000000\n"));
    assert!(!rendered.contains("Protocol issue"));
    assert!(rendered.contains(r#""added_later":7"#));
}

#[test]
fn known_code_with_a_spoofed_type_is_called_out_explicitly() {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let received = receive(
        br#"{"type":"https://attacker.invalid/problem","status":404,"code":"DSP-1003","evidence":{}}"#,
        StatusCode::NOT_FOUND,
    );
    let rendered = render_problem(&old_catalog, &received)
        .unwrap_or_else(|error| panic!("spoofed fixture must render: {error}"));

    assert!(rendered.starts_with("DSP-1003 — Job not found\nHTTP 404\n"));
    assert!(rendered.contains(
        "Protocol issue: type URI https://attacker.invalid/problem differs from https://dispatch.invalid/problems/DSP-1003"
    ));
}

#[test]
fn malformed_identity_uses_an_explicit_fallback_without_losing_it() {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    let received = receive(
        br#"{"code":17,"title":"untyped future failure","future":true}"#,
        StatusCode::INTERNAL_SERVER_ERROR,
    );
    let rendered = render_problem(&old_catalog, &received)
        .unwrap_or_else(|error| panic!("fallback fixture must render: {error}"));

    assert!(rendered.starts_with("Unknown diagnostic <unrecognized>\nHTTP 500\n"));
    assert!(rendered.contains(r#""code":17"#));
    assert!(rendered.contains(r#""future":true"#));
}
