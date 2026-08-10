//! Old-client rendering of durable diagnostics recorded after acceptance.

use dispatch_cli::render_operation;
use dispatch_diagnostics::catalog;
use recourse::client::{DecodeLimits, ReceivedOperationDiagnostic};

fn receive(body: &[u8]) -> ReceivedOperationDiagnostic {
    ReceivedOperationDiagnostic::from_slice(body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("bounded fixture must decode: {error}"))
}

fn render(body: &[u8]) -> String {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    render_operation(&old_catalog, &receive(body))
        .unwrap_or_else(|error| panic!("fixture must render: {error}"))
}

#[test]
fn a_known_durable_failure_shows_local_identity_evidence_and_impact() {
    let rendered = render(include_bytes!(
        "../../../conformance/wire/dispatch-operation.json"
    ));

    assert!(rendered.starts_with(
        "DSP-1009 — Job dispatch failed\nDiagnostic dia_01K00000000000000000000000-3\n"
    ));
    assert!(rendered.contains("Detail: The job was accepted but could not be dispatched.\n"));
    assert!(
        rendered.contains(r#"Evidence: {"attempt":3,"job_id":"job_01K00000000000000000000000"}"#)
    );
    assert!(rendered.contains(r#"Impact: {"created_artifacts":2,"destination_changed":false}"#));
    assert!(rendered.contains("Suggestion: Inspect the failed attempt.\n"));
    assert!(!rendered.contains("HTTP "));
}

#[test]
fn a_newer_durable_code_survives_with_hostile_display_text() {
    let rendered = render(
        br#"{"id":"dia_01K00000000000000000000000-9","type":"https://dispatch.invalid/problems/DSP-1998","code":"DSP-1998","title":"\u001b[31mFuture failure\u001b[0m","detail":"first\n\u202esecond","impact":{"future":{"nested":[1,2,3]}},"vendor":"kept"}"#,
    );

    assert!(rendered.starts_with(
        "Unknown durable diagnostic DSP-1998\nDiagnostic dia_01K00000000000000000000000-9\n"
    ));
    assert!(rendered.contains("Title: \\u{1b}[31mFuture failure\\u{1b}[0m"));
    assert!(rendered.contains("first\\n\\u{202e}second"));
    assert!(rendered.contains(r#""future":{"nested":[1,2,3]}"#));
    assert!(rendered.contains(r#""vendor":"kept""#));
    assert!(!rendered.contains('\u{1b}'));
    assert!(!rendered.contains('\u{202e}'));
}

#[test]
fn a_malformed_occurrence_identity_is_reported_without_losing_data() {
    let rendered = render(
        br#"{"id":"01K-not-a-diagnostic","code":"DSP-1009","type":"https://attacker.invalid/problem","impact":{}}"#,
    );

    assert!(rendered.starts_with("DSP-1009 — Job dispatch failed\nDiagnostic <unrecognized>\n"));
    assert!(rendered.contains(
        "Protocol issue: type URI https://attacker.invalid/problem differs from https://dispatch.invalid/problems/DSP-1009"
    ));
    assert!(rendered.contains("Protocol issue: malformed operation diagnostic ID"));
    assert!(rendered.contains(r#""id":"01K-not-a-diagnostic""#));
}

#[test]
fn a_problem_only_code_is_not_treated_as_a_durable_surface() {
    let rendered = render(br#"{"id":"dia_1","code":"DSP-1003","title":"Job not found"}"#);

    assert!(rendered.starts_with("Unknown durable diagnostic DSP-1003\nDiagnostic dia_1\n"));
    assert!(rendered.contains("Title: Job not found\n"));
}
