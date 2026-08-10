//! Focused tests for private error ownership and ordered context.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use super::PrivateReport;

#[derive(Debug)]
struct DatabaseError;

impl Display for DatabaseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("password=private-canary")
    }
}

impl Error for DatabaseError {}

#[test]
fn report_keeps_source_and_context_private_and_ordered() {
    let report = PrivateReport::new(DatabaseError)
        .context("operation", "load_job")
        .context("job_id", "job_private");

    assert_eq!(report.source_error().to_string(), "password=private-canary");
    assert_eq!(report.contexts().len(), 2);
    assert_eq!(report.contexts()[0].key(), "operation");
    assert_eq!(report.contexts()[1].value(), "job_private");
    assert!(report.to_string().contains("password=private-canary"));
}
