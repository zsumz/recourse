//! The reference fault reporter writes operator-only structured lines.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io::{self, Write},
    str::FromStr,
    sync::{Arc, Mutex, PoisonError},
};

use axum::http::{Method, StatusCode};
use dispatch_api_axum::FaultLog;
use recourse::{
    catalog::Code,
    fault::PrivateReport,
    http::{CorrelationId, ProblemOccurrence},
    observe::{FaultEvent, FaultReporter, HttpEventContext, NormalizedRoute},
};

const PRIVATE_STORE: &str = "postgres://dispatch:PRIVATE_STORAGE_TOKEN_9ba2@jobs.internal";

#[derive(Debug, Clone, Default)]
struct CapturedLines(Arc<Mutex<Vec<u8>>>);

impl CapturedLines {
    fn text(&self) -> String {
        let bytes = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Write for CapturedLines {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn storage_fault() -> (FaultEvent, PrivateReport) {
    let correlation_id = CorrelationId::new("fault-log-request")
        .unwrap_or_else(|error| panic!("fixture request ID must be valid: {error}"));
    let occurrence = ProblemOccurrence::new(
        correlation_id,
        "https://api.dispatch.invalid/problem-occurrences/fault-log-request",
    )
    .unwrap_or_else(|error| panic!("fixture occurrence must be valid: {error}"));
    let code = Code::from_str("DSP-1008")
        .unwrap_or_else(|error| panic!("fixture code must parse: {error}"));
    let route = NormalizedRoute::new("/jobs")
        .unwrap_or_else(|error| panic!("fixture route must be valid: {error}"));
    let context = HttpEventContext::new()
        .with_method(Method::POST)
        .with_route(route);
    let event = FaultEvent::for_http(
        code,
        StatusCode::INTERNAL_SERVER_ERROR,
        &occurrence,
        &context,
        false,
    );
    let report = PrivateReport::new(io::Error::other("connect to job store"))
        .context("operation", "create_job")
        .context("store", PRIVATE_STORE);
    (event, report)
}

#[test]
fn one_fault_becomes_one_structured_operator_line() {
    let sink = CapturedLines::default();
    let reporter = FaultLog::new(sink.clone());
    let (event, report) = storage_fault();

    reporter.report_fault(&event, &report);
    let text = sink.text();

    assert_eq!(text.lines().count(), 1);
    assert!(text.starts_with("dispatch.fault code=DSP-1008 status=500 "));
    assert!(text.contains("request_id=fault-log-request"));
    assert!(text.contains("method=POST route=/jobs fallback=false"));
    assert!(text.ends_with('\n'));
}

#[test]
fn the_private_report_reaches_operators_and_nothing_else_is_added() {
    let sink = CapturedLines::default();
    let reporter = FaultLog::new(sink.clone());
    let (event, report) = storage_fault();

    reporter.report_fault(&event, &report);
    reporter.report_fault(&event, &report);
    let text = sink.text();

    assert_eq!(text.lines().count(), 2);
    assert!(text.contains(PRIVATE_STORE));
    assert!(text.contains("[operation=create_job]"));
    assert!(text.contains("connect to job store"));
}

/// Source error whose text tries to end the operator line and drive the
/// terminal, which is reachable through a panic payload or a dependency error.
#[derive(Debug)]
struct ForgedFaultText;

impl Display for ForgedFaultText {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "connect failed\ndispatch.fault code=DSP-0001 forged=1\u{1b}[31mred\u{1b}]8;;https://attacker.invalid\u{7}",
        )
    }
}

impl Error for ForgedFaultText {}

#[test]
fn hostile_source_text_cannot_forge_a_line_or_drive_the_terminal() {
    let sink = CapturedLines::default();
    let reporter = FaultLog::new(sink.clone());
    let (event, _) = storage_fault();
    let report = PrivateReport::new(ForgedFaultText).context("operation", "create_job");

    reporter.report_fault(&event, &report);
    let text = sink.text();

    assert_eq!(text.lines().count(), 1);
    assert!(!text.contains('\u{1b}'));
    assert!(text.contains("connect failed\\ndispatch.fault code=DSP-0001 forged=1"));
    assert!(text.contains("\\u{1b}[31mred\\u{1b}]8;;https://attacker.invalid\\u{7}"));
    assert!(text.ends_with('\n'));
}
