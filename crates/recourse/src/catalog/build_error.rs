//! Aggregation of independently actionable catalog definition issues.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use super::CatalogIssue;

/// All definition failures found during one catalog build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogBuildError {
    issues: Vec<CatalogIssue>,
}

impl CatalogBuildError {
    pub(crate) fn new(issues: Vec<CatalogIssue>) -> Self {
        Self { issues }
    }

    /// Independently actionable issues in deterministic discovery order.
    pub fn issues(&self) -> &[CatalogIssue] {
        &self.issues
    }
}

impl Display for CatalogBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "catalog construction found {} issue(s)",
            self.issues.len()
        )?;
        for issue in &self.issues {
            writeln!(formatter, "- {issue}")?;
        }
        Ok(())
    }
}

impl Error for CatalogBuildError {}
