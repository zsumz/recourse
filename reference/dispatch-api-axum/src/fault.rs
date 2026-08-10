//! Operator-facing private fault reporting for the reference API.

use std::{
    io::{self, Stderr, Write},
    sync::{Mutex, PoisonError},
};

use recourse::{
    client::escape_terminal,
    fault::PrivateReport,
    observe::{FaultEvent, FaultReporter},
};

/// Writes one structured line per unexpected fault to an operator sink.
///
/// The line carries the bounded public metadata Recourse supplies plus the
/// private report, which is why the sink must be an operator-only destination
/// rather than anything a caller can read.
#[derive(Debug)]
pub struct FaultLog<W: Write + Send + 'static> {
    sink: Mutex<W>,
}

impl FaultLog<Stderr> {
    /// Reports faults on the process standard error stream.
    pub fn to_stderr() -> Self {
        Self::new(io::stderr())
    }
}

impl<W: Write + Send + 'static> FaultLog<W> {
    /// Reports faults to an application-supplied sink.
    pub const fn new(sink: W) -> Self {
        Self {
            sink: Mutex::new(sink),
        }
    }
}

impl<W: Write + Send + 'static> FaultReporter for FaultLog<W> {
    fn report_fault(&self, event: &FaultEvent, report: &PrivateReport) {
        let line = fault_line(event, report);
        let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
        // Reporting runs after the response is decided, so a failed write must
        // never replace the sanitized Problem the caller already receives.
        drop(writeln!(sink, "{line}"));
    }
}

fn fault_line(event: &FaultEvent, report: &PrivateReport) -> String {
    let metadata = event.problem_metadata();
    let method = metadata
        .request_method()
        .map_or("-", axum::http::Method::as_str);
    let route = metadata
        .normalized_route()
        .map_or("-", recourse::observe::NormalizedRoute::as_str);
    // The report renders source-error text and private context verbatim, so a
    // hostile payload could otherwise forge a second operator line or drive the
    // terminal. Escaping renders every control character visibly, including the
    // newline that would end this line, which keeps one fault on one line.
    let rendered = escape_terminal(&report.to_string());
    format!(
        "dispatch.fault code={} status={} request_id={} method={method} route={route} \
         fallback={} report={rendered}",
        metadata.code(),
        metadata.status().as_u16(),
        metadata.correlation_id(),
        metadata.used_fallback_encoding(),
    )
}
