//! Tolerant current health finding preserving unknown remote data.

use serde_json::{Map, Value};

use crate::{
    catalog::{Catalog, CatalogSpec, Code},
    health::{HealthFindingId, HealthSeverity, ObservationTime},
};

use super::{Classification, DecodeError, DecodeLimits, ProtocolIssue, decode_object, member};

/// Tolerantly decoded current service-state finding.
#[derive(Debug, Clone)]
pub struct ReceivedHealthFinding {
    id: Option<HealthFindingId>,
    type_uri: Option<String>,
    code: Option<Code>,
    title: Option<String>,
    detail: Option<String>,
    severity: Option<HealthSeverity>,
    observed_at: Option<ObservationTime>,
    evidence: Option<Map<String, Value>>,
    suggestions: Vec<String>,
    raw: Map<String, Value>,
    issues: Vec<ProtocolIssue>,
}

impl ReceivedHealthFinding {
    /// Decodes untrusted health finding JSON under explicit limits.
    pub fn from_slice(body: &[u8], limits: DecodeLimits) -> Result<Self, DecodeError> {
        let raw = decode_object(body, limits)?;
        let mut issues = Vec::new();
        let id = parse_id(&raw, &mut issues);
        let code = member::code(&raw, &mut issues);
        let severity = parse_severity(&raw, &mut issues);
        let observed_at = parse_observed_at(&raw, &mut issues);
        Ok(Self {
            id,
            type_uri: member::string(&raw, "type"),
            code,
            title: member::string(&raw, "title"),
            detail: member::string(&raw, "detail"),
            severity,
            observed_at,
            evidence: member::object(&raw, "evidence"),
            suggestions: member::string_array(&raw, "suggestions"),
            raw,
            issues,
        })
    }

    /// Valid finding occurrence ID when supplied correctly.
    pub const fn id(&self) -> Option<&HealthFindingId> {
        self.id.as_ref()
    }

    /// String-valued semantic type member when supplied correctly.
    pub fn type_uri(&self) -> Option<&str> {
        self.type_uri.as_deref()
    }

    /// Canonical diagnostic code when supplied correctly.
    pub const fn code(&self) -> Option<&Code> {
        self.code.as_ref()
    }

    /// Optional human-readable title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Optional human-readable detail.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Closed health severity when supplied correctly.
    pub const fn severity(&self) -> Option<HealthSeverity> {
        self.severity
    }

    /// Valid RFC 3339 observation time when supplied correctly.
    pub const fn observed_at(&self) -> Option<&ObservationTime> {
        self.observed_at.as_ref()
    }

    /// Object-valued public evidence when supplied correctly.
    pub const fn evidence(&self) -> Option<&Map<String, Value>> {
        self.evidence.as_ref()
    }

    /// String-valued suggestions retained in sender order.
    pub fn suggestions(&self) -> &[String] {
        &self.suggestions
    }

    /// Complete raw object, including unknown and wrong-typed members.
    pub const fn raw(&self) -> &Map<String, Value> {
        &self.raw
    }

    /// Nonfatal inconsistencies discovered during tolerant parsing.
    pub fn protocol_issues(&self) -> &[ProtocolIssue] {
        &self.issues
    }
}

impl<C: CatalogSpec> Catalog<C> {
    /// Classifies a received finding against registered health surfaces.
    pub fn classify_health<'a>(&'a self, received: &ReceivedHealthFinding) -> Classification<'a> {
        let definition = received.code().and_then(|code| self.diagnostic(code));
        definition
            .filter(|value| value.supports_health())
            .map_or(Classification::Unknown, Classification::Known)
    }
}

fn parse_id(raw: &Map<String, Value>, issues: &mut Vec<ProtocolIssue>) -> Option<HealthFindingId> {
    let value = raw.get("id")?.as_str()?;
    if let Ok(id) = HealthFindingId::try_new(value) {
        Some(id)
    } else {
        issues.push(ProtocolIssue::MalformedHealthFindingId);
        None
    }
}

fn parse_severity(
    raw: &Map<String, Value>,
    issues: &mut Vec<ProtocolIssue>,
) -> Option<HealthSeverity> {
    let value = raw.get("severity")?.as_str()?;
    match value {
        "degraded" => Some(HealthSeverity::Degraded),
        "unhealthy" => Some(HealthSeverity::Unhealthy),
        _ => {
            issues.push(ProtocolIssue::InvalidHealthSeverity);
            None
        }
    }
}

fn parse_observed_at(
    raw: &Map<String, Value>,
    issues: &mut Vec<ProtocolIssue>,
) -> Option<ObservationTime> {
    let value = raw.get("observed_at")?.as_str()?;
    if let Ok(observed_at) = ObservationTime::parse(value) {
        Some(observed_at)
    } else {
        issues.push(ProtocolIssue::InvalidObservationTime);
        None
    }
}
