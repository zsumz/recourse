//! Catalog-aware classification with explicit HTTP conformance findings.

mod health;
mod operation;

use http::StatusCode;
use serde_json::{Map, Value};

use crate::catalog::{Catalog, CatalogDiagnostic, CatalogSpec};

use super::{ProtocolIssue, ReceivedProblem};

pub use health::{HealthClassification, KnownHealthClassification};
pub use operation::{KnownOperationClassification, OperationClassification};

/// Classification against one explicitly built local catalog.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Classification<'a> {
    /// Received code is present in the requested local surface.
    Known(&'a CatalogDiagnostic),
    /// Code is absent, malformed, or newer than the local catalog.
    Unknown,
}

fn envelope_issues(
    diagnostic: &CatalogDiagnostic,
    type_uri: Option<&str>,
    raw: &Map<String, Value>,
    contract: EnvelopeContract,
) -> Vec<ProtocolIssue> {
    let mut issues = Vec::new();
    if type_uri != Some(diagnostic.type_uri()) {
        issues.push(ProtocolIssue::UnexpectedTypeForCode {
            expected: diagnostic.type_uri().to_owned(),
            received: type_uri.map(str::to_owned),
        });
    }
    if !contract.registered {
        issues.push(contract.surface_issue);
    }
    for member in contract.required {
        if !raw.contains_key(*member) {
            issues.push(ProtocolIssue::MissingRequiredMember { member });
        }
    }
    issues
}

struct EnvelopeContract {
    registered: bool,
    surface_issue: ProtocolIssue,
    required: &'static [&'static str],
}

/// HTTP Problem classification with catalog-aware conformance findings.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ProblemClassification<'a> {
    /// Received code is present, with its declaration conformance result.
    Known(KnownProblemClassification<'a>),
    /// Code is absent, malformed, or newer than the local catalog.
    Unknown,
}

/// Known diagnostic plus every transport/declaration inconsistency.
#[derive(Debug, Clone)]
pub struct KnownProblemClassification<'a> {
    diagnostic: &'a CatalogDiagnostic,
    problem: &'a ReceivedProblem,
    catalog_issues: Vec<ProtocolIssue>,
}

impl KnownProblemClassification<'_> {
    /// Matching local diagnostic declaration.
    pub const fn diagnostic(&self) -> &CatalogDiagnostic {
        self.diagnostic
    }

    /// Issues that required the local catalog to discover.
    pub fn catalog_issues(&self) -> &[ProtocolIssue] {
        &self.catalog_issues
    }

    /// Parsing and catalog-aware issues considered together.
    pub fn protocol_issues(&self) -> impl Iterator<Item = &ProtocolIssue> {
        self.problem
            .protocol_issues()
            .iter()
            .chain(self.catalog_issues.iter())
    }

    /// Whether identity, transport status, and required headers conform.
    pub fn is_conformant(&self) -> bool {
        self.protocol_issues().next().is_none()
    }
}

impl<C: CatalogSpec> Catalog<C> {
    /// Classifies by permanent code and checks its complete HTTP declaration.
    pub fn classify<'a>(&'a self, problem: &'a ReceivedProblem) -> ProblemClassification<'a> {
        let Some(diagnostic) = problem.code().and_then(|code| self.diagnostic(code)) else {
            return ProblemClassification::Unknown;
        };
        ProblemClassification::Known(KnownProblemClassification {
            diagnostic,
            problem,
            catalog_issues: catalog_issues(diagnostic, problem),
        })
    }
}

fn catalog_issues(diagnostic: &CatalogDiagnostic, problem: &ReceivedProblem) -> Vec<ProtocolIssue> {
    let mut issues = Vec::new();
    if problem.type_uri() != Some(diagnostic.type_uri()) {
        issues.push(ProtocolIssue::UnexpectedTypeForCode {
            expected: diagnostic.type_uri().to_owned(),
            received: problem.type_uri().map(str::to_owned),
        });
    }
    let Some(expected) = diagnostic
        .http_status()
        .and_then(|status| StatusCode::from_u16(status).ok())
    else {
        issues.push(ProtocolIssue::CodeNotRegisteredForHttp);
        return issues;
    };
    if problem.transport_status() != expected {
        issues.push(ProtocolIssue::CatalogStatusMismatch {
            expected,
            transport: problem.transport_status(),
        });
    }
    for header in diagnostic.required_headers().unwrap_or_default() {
        if !problem.headers().contains_key(header) {
            issues.push(ProtocolIssue::MissingRequiredHeader {
                header: header.clone(),
            });
        }
    }
    issues
}
