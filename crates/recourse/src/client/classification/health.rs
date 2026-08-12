//! Governed classification of received health findings.

use crate::catalog::{Catalog, CatalogDiagnostic, CatalogSpec};

use super::{EnvelopeContract, envelope_issues};
use crate::client::{ProtocolIssue, ReceivedHealthFinding};

const REQUIRED_MEMBERS: &[&str] = &[
    "id",
    "title",
    "detail",
    "severity",
    "observed_at",
    "evidence",
    "suggestions",
];

/// Health-finding classification with catalog-aware conformance findings.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HealthClassification<'a> {
    /// Received code is present, with its declaration conformance result.
    Known(KnownHealthClassification<'a>),
    /// Code is absent, malformed, or newer than the local catalog.
    Unknown,
}

/// Known health finding plus every envelope/declaration inconsistency.
#[derive(Debug, Clone)]
pub struct KnownHealthClassification<'a> {
    diagnostic: &'a CatalogDiagnostic,
    received: &'a ReceivedHealthFinding,
    catalog_issues: Vec<ProtocolIssue>,
}

impl KnownHealthClassification<'_> {
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
    /// Classifies by permanent code and checks the health envelope contract.
    pub fn classify_health_conformance<'a>(
        &'a self,
        received: &'a ReceivedHealthFinding,
    ) -> HealthClassification<'a> {
        let Some(diagnostic) = received.code().and_then(|code| self.diagnostic(code)) else {
            return HealthClassification::Unknown;
        };
        let catalog_issues = envelope_issues(
            diagnostic,
            received.type_uri(),
            received.raw(),
            EnvelopeContract {
                registered: diagnostic.supports_health(),
                surface_issue: ProtocolIssue::CodeNotRegisteredForHealth,
                required: REQUIRED_MEMBERS,
            },
        );
        HealthClassification::Known(KnownHealthClassification {
            diagnostic,
            received,
            catalog_issues,
        })
    }
}
