//! Tolerant durable-operation diagnostic preserving unknown remote data.

use serde_json::{Map, Value};

use crate::{
    catalog::{Catalog, CatalogSpec, Code},
    operation::OperationDiagnosticId,
};

use super::{Classification, DecodeError, DecodeLimits, ProtocolIssue, decode_object, member};

/// Tolerantly decoded durable operation diagnostic.
#[derive(Debug, Clone)]
pub struct ReceivedOperationDiagnostic {
    id: Option<OperationDiagnosticId>,
    type_uri: Option<String>,
    code: Option<Code>,
    title: Option<String>,
    detail: Option<String>,
    evidence: Option<Map<String, Value>>,
    impact: Option<Map<String, Value>>,
    suggestions: Vec<String>,
    raw: Map<String, Value>,
    issues: Vec<ProtocolIssue>,
}

impl ReceivedOperationDiagnostic {
    /// Decodes untrusted durable diagnostic JSON under explicit limits.
    pub fn from_slice(body: &[u8], limits: DecodeLimits) -> Result<Self, DecodeError> {
        let raw = decode_object(body, limits)?;
        let mut issues = Vec::new();
        let id = parse_id(&raw, &mut issues);
        let code = member::code(&raw, &mut issues);
        Ok(Self {
            id,
            type_uri: member::string(&raw, "type", &mut issues),
            code,
            title: member::string(&raw, "title", &mut issues),
            detail: member::string(&raw, "detail", &mut issues),
            evidence: member::object(&raw, "evidence", &mut issues),
            impact: member::object(&raw, "impact", &mut issues),
            suggestions: member::string_array(&raw, "suggestions", &mut issues),
            raw,
            issues,
        })
    }

    /// Valid durable diagnostic occurrence ID when supplied correctly.
    pub const fn id(&self) -> Option<&OperationDiagnosticId> {
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

    /// Object-valued public evidence when supplied correctly.
    pub const fn evidence(&self) -> Option<&Map<String, Value>> {
        self.evidence.as_ref()
    }

    /// Object-valued public impact when supplied correctly.
    pub const fn impact(&self) -> Option<&Map<String, Value>> {
        self.impact.as_ref()
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
    /// Looks up a received diagnostic by code on the operation surface.
    ///
    /// Use [`Catalog::classify_operation_conformance`] when identity and
    /// required envelope-member findings are needed.
    pub fn classify_operation<'a>(
        &'a self,
        received: &ReceivedOperationDiagnostic,
    ) -> Classification<'a> {
        let definition = received.code().and_then(|code| self.diagnostic(code));
        definition
            .filter(|value| value.impact_schema().is_some())
            .map_or(Classification::Unknown, Classification::Known)
    }
}

fn parse_id(
    raw: &Map<String, Value>,
    issues: &mut Vec<ProtocolIssue>,
) -> Option<OperationDiagnosticId> {
    let value = member::string(raw, "id", issues)?;
    if let Ok(id) = OperationDiagnosticId::try_new(&value) {
        Some(id)
    } else {
        issues.push(ProtocolIssue::MalformedOperationDiagnosticId);
        None
    }
}
