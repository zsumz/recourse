//! Tolerant HTTP Problem representation preserving the complete raw object.

use http::{HeaderMap, StatusCode};
use serde_json::{Map, Value};

use crate::catalog::{Catalog, CatalogDiagnostic, CatalogSpec, Code};

use super::{DecodeError, DecodeLimits, ProtocolIssue, decode_object, member};

/// Tolerantly decoded remote HTTP Problem with authoritative transport facts.
#[derive(Debug, Clone)]
pub struct ReceivedProblem {
    transport_status: StatusCode,
    headers: HeaderMap,
    type_uri: Option<String>,
    title: Option<String>,
    detail: Option<String>,
    instance: Option<String>,
    code: Option<Code>,
    body_status: Option<StatusCode>,
    evidence: Option<Map<String, Value>>,
    suggestions: Vec<String>,
    raw: Map<String, Value>,
    issues: Vec<ProtocolIssue>,
}

impl ReceivedProblem {
    /// Decodes a remote Problem under explicit resource limits.
    ///
    /// Decoding is tolerant and bounded: malformed JSON produces an error
    /// rather than a panic, unknown members survive in [`raw`](Self::raw), and
    /// disagreements are recorded as
    /// [`protocol_issues`](Self::protocol_issues) instead of failing. Classify
    /// against a local catalog to decide how to render:
    ///
    /// ```
    /// use recourse::{
    ///     catalog::{Catalog, CatalogSpec, CodeNumber},
    ///     client::{Classification, DecodeLimits, ReceivedProblem},
    ///     diagnostic::{DiagnosticType, NoEvidence},
    ///     http::{Fixed, HttpProblemType},
    ///     dependencies::http::{HeaderMap, StatusCode},
    /// };
    ///
    /// # enum ServiceCatalog {}
    /// # impl CatalogSpec for ServiceCatalog {
    /// #     const NAME: &'static str = "example-service";
    /// #     const PREFIX: &'static str = "EXM";
    /// #     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
    /// # }
    /// # enum ResourceMissing {}
    /// # impl DiagnosticType for ResourceMissing {
    /// #     type Catalog = ServiceCatalog;
    /// #     type Evidence = NoEvidence;
    /// #     const NUMBER: CodeNumber = CodeNumber::new(1001);
    /// #     const TITLE: &'static str = "Resource missing";
    /// #     const DETAIL: &'static str = "The requested resource does not exist.";
    /// #     const SUGGESTIONS: &'static [&'static str] = &["Check the identifier."];
    /// #     const DOCS: &'static str = "Verify the identifier before retrying.";
    /// # }
    /// # impl HttpProblemType for ResourceMissing { type Policy = Fixed<404>; }
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let catalog = Catalog::<ServiceCatalog>::builder()
    ///     .problem::<ResourceMissing>()
    ///     .build()?;
    /// // A newer server sends a code this build has never heard of.
    /// let body = br#"{"code":"EXM-1999","title":"Future failure","vendor":"kept"}"#;
    /// let received = ReceivedProblem::from_slice(
    ///     StatusCode::BAD_GATEWAY,
    ///     &HeaderMap::new(),
    ///     body,
    ///     DecodeLimits::default(),
    /// )?;
    ///
    /// assert!(matches!(catalog.classify(&received), Classification::Unknown));
    /// assert_eq!(received.transport_status(), StatusCode::BAD_GATEWAY);
    /// assert!(received.raw().contains_key("vendor"));
    /// # Ok(())
    /// # }
    /// # assert!(example().is_ok());
    /// ```
    pub fn from_slice(
        transport_status: StatusCode,
        headers: &HeaderMap,
        body: &[u8],
        limits: DecodeLimits,
    ) -> Result<Self, DecodeError> {
        let raw = decode_object(body, limits)?;
        let mut issues = Vec::new();
        let code = member::code(&raw, &mut issues);
        let body_status = parse_status(&raw, &mut issues);
        if let Some(body) = body_status.filter(|body| *body != transport_status) {
            issues.push(ProtocolIssue::TransportStatusMismatch {
                transport: transport_status,
                body,
            });
        }
        Ok(Self {
            transport_status,
            headers: headers.clone(),
            type_uri: member::string(&raw, "type"),
            title: member::string(&raw, "title"),
            detail: member::string(&raw, "detail"),
            instance: member::string(&raw, "instance"),
            code,
            body_status,
            evidence: member::object(&raw, "evidence"),
            suggestions: member::string_array(&raw, "suggestions"),
            raw,
            issues,
        })
    }

    /// Authoritative actual HTTP response status.
    pub const fn transport_status(&self) -> StatusCode {
        self.transport_status
    }

    /// Complete validated HTTP response headers supplied by the transport.
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// String-valued RFC 9457 type member when present.
    pub fn type_uri(&self) -> Option<&str> {
        self.type_uri.as_deref()
    }

    /// Optional human-readable title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Optional human-readable detail.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Optional occurrence URI-reference text.
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_deref()
    }

    /// Canonical parsed diagnostic code when supplied correctly.
    pub const fn code(&self) -> Option<&Code> {
        self.code.as_ref()
    }

    /// Valid body status when supplied, even if it disagrees with transport.
    pub const fn body_status(&self) -> Option<StatusCode> {
        self.body_status
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

/// Classification against one explicitly built local catalog.
#[derive(Debug, Clone, Copy)]
pub enum Classification<'a> {
    /// Received code is present in the local catalog.
    Known(&'a CatalogDiagnostic),
    /// Code is absent, malformed, or newer than the local catalog.
    Unknown,
}

impl<C: CatalogSpec> Catalog<C> {
    /// Classifies by permanent code without rejecting newer definitions.
    pub fn classify<'a>(&'a self, problem: &ReceivedProblem) -> Classification<'a> {
        let Some(code) = problem.code() else {
            return Classification::Unknown;
        };
        self.diagnostic(code)
            .map_or(Classification::Unknown, Classification::Known)
    }
}

fn parse_status(raw: &Map<String, Value>, issues: &mut Vec<ProtocolIssue>) -> Option<StatusCode> {
    let value = raw.get("status")?.as_u64()?;
    let status = u16::try_from(value)
        .ok()
        .and_then(|value| StatusCode::from_u16(value).ok());
    if status.is_none() {
        issues.push(ProtocolIssue::InvalidBodyStatus);
    }
    status
}
