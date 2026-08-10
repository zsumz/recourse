//! Old-client rendering of current health findings from a newer service.

use dispatch_cli::render_health;
use dispatch_diagnostics::catalog;
use recourse::client::{DecodeLimits, ReceivedHealthFinding};

fn receive(body: &[u8]) -> ReceivedHealthFinding {
    ReceivedHealthFinding::from_slice(body, DecodeLimits::default())
        .unwrap_or_else(|error| panic!("bounded fixture must decode: {error}"))
}

fn render(body: &[u8]) -> String {
    let old_catalog = catalog().unwrap_or_else(|error| panic!("catalog must build: {error}"));
    render_health(&old_catalog, &receive(body))
        .unwrap_or_else(|error| panic!("fixture must render: {error}"))
}

#[test]
fn a_known_finding_shows_local_identity_severity_and_observation() {
    let rendered = render(include_bytes!(
        "../../../conformance/wire/dispatch-health-finding.json"
    ));

    assert!(
        rendered
            .starts_with("DSP-1010 — Job queue unavailable\nFinding finding_queue-unavailable\n")
    );
    assert!(rendered.contains("Severity: degraded\n"));
    assert!(rendered.contains("Observed: 2026-08-10T14:31:00Z\n"));
    assert!(rendered.contains(r#"Evidence: {"consecutive_failures":3}"#));
    assert!(rendered.contains("Suggestion: Check queue connectivity.\n"));
    assert!(!rendered.contains("HTTP "));
}

#[test]
fn a_newer_finding_code_survives_with_hostile_display_text() {
    let rendered = render(
        br#"{"id":"finding_future","code":"DSP-1997","title":"\u001b[31mFuture state\u001b[0m","severity":"unhealthy","observed_at":"2026-08-10T14:31:00Z","detail":"first\n\u202esecond","vendor":"kept"}"#,
    );

    assert!(rendered.starts_with("Unknown finding DSP-1997\nFinding finding_future\n"));
    assert!(rendered.contains("Severity: unhealthy\n"));
    assert!(rendered.contains("Title: \\u{1b}[31mFuture state\\u{1b}[0m"));
    assert!(rendered.contains("first\\n\\u{202e}second"));
    assert!(rendered.contains(r#""vendor":"kept""#));
    assert!(!rendered.contains('\u{1b}'));
    assert!(!rendered.contains('\u{202e}'));
}

#[test]
fn an_unreadable_severity_and_time_are_reported_without_losing_data() {
    let rendered = render(
        br#"{"id":"finding_queue-unavailable","code":"DSP-1010","type":"https://dispatch.invalid/problems/DSP-1010","severity":"catastrophic","observed_at":"yesterday"}"#,
    );

    assert!(rendered.contains("Severity: <unrecognized>\n"));
    assert!(!rendered.contains("Observed: "));
    assert!(rendered.contains("Protocol issue: invalid health severity\n"));
    assert!(rendered.contains("Protocol issue: invalid health observation time\n"));
    assert!(rendered.contains(r#""severity":"catastrophic""#));
    assert!(rendered.contains(r#""observed_at":"yesterday""#));
}

#[test]
fn a_problem_only_code_is_not_treated_as_a_health_surface() {
    let rendered = render(br#"{"id":"finding_x","code":"DSP-1003","severity":"degraded"}"#);

    assert!(rendered.starts_with("Unknown finding DSP-1003\nFinding finding_x\n"));
}
