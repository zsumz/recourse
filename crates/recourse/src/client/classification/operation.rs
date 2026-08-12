//! Governed classification of received durable-operation diagnostics.

use crate::catalog::{Catalog, CatalogDiagnostic, CatalogSpec};

use super::{EnvelopeContract, envelope_issues};
use crate::client::{ProtocolIssue, ReceivedOperationDiagnostic};

const REQUIRED_MEMBERS: &[&str] = &["id", "title", "detail", "evidence", "impact", "suggestions"];

/// Durable-operation classification with catalog-aware conformance findings.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum OperationClassification<'a> {
    /// Received code is present, with its declaration conformance result.
    Known(KnownOperationClassification<'a>),
    /// Code is absent, malformed, or newer than the local catalog.
    Unknown,
}

/// Known operation diagnostic plus every envelope/declaration inconsistency.
#[derive(Debug, Clone)]
pub struct KnownOperationClassification<'a> {
    diagnostic: &'a CatalogDiagnostic,
    received: &'a ReceivedOperationDiagnostic,
    catalog_issues: Vec<ProtocolIssue>,
}

impl KnownOperationClassification<'_> {
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
        self.received
            .protocol_issues()
            .iter()
            .chain(self.catalog_issues.iter())
    }

    /// Whether identity, surface registration, and required members conform.
    pub fn is_conformant(&self) -> bool {
        self.protocol_issues().next().is_none()
    }
}

impl<C: CatalogSpec> Catalog<C> {
    /// Classifies by permanent code and checks the operation envelope contract.
    pub fn classify_operation_conformance<'a>(
        &'a self,
        received: &'a ReceivedOperationDiagnostic,
    ) -> OperationClassification<'a> {
        let Some(diagnostic) = received.code().and_then(|code| self.diagnostic(code)) else {
            return OperationClassification::Unknown;
        };
        let catalog_issues = envelope_issues(
            diagnostic,
            received.type_uri(),
            received.raw(),
            EnvelopeContract {
                registered: diagnostic.impact_schema().is_some(),
                surface_issue: ProtocolIssue::CodeNotRegisteredForOperation,
                required: REQUIRED_MEMBERS,
            },
        );
        OperationClassification::Known(KnownOperationClassification {
            diagnostic,
            received,
            catalog_issues,
        })
    }
}
