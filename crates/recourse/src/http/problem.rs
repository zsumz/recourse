//! Strict typed Problem construction and canonical JSON encoding.

mod error;

use std::any::type_name;

use http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE};
use serde::Serialize;

use crate::{
    catalog::{Catalog, CatalogDiagnostic, CatalogSpec, Code},
    diagnostic::{PublicEvidence, PublicText},
};

use super::{EncodedProblem, HttpPolicy, HttpProblemType, ProblemOccurrence};

pub use error::{ProblemBuildError, ProblemEncodeError};

const PROBLEM_JSON: HeaderValue = HeaderValue::from_static("application/problem+json");

/// Typed public Problem awaiting canonical encoding.
#[derive(Debug)]
pub struct Problem<E: PublicEvidence> {
    type_uri: String,
    title: String,
    status: StatusCode,
    detail: String,
    occurrence: ProblemOccurrence,
    code: Code,
    evidence: E,
    suggestions: Vec<String>,
    headers: HeaderMap,
}

impl<E: PublicEvidence> Problem<E> {
    fn from_parts(parts: ProblemParts<E>) -> Self {
        Self {
            type_uri: parts.definition.type_uri().to_owned(),
            title: parts.definition.title().to_owned(),
            status: parts.status,
            detail: parts.detail.map_or_else(
                || parts.definition.detail().to_owned(),
                |value| value.to_string(),
            ),
            occurrence: parts.occurrence,
            code: parts.definition.code().clone(),
            evidence: parts.evidence,
            suggestions: parts.definition.suggestions().to_vec(),
            headers: parts.headers,
        }
    }

    /// Governed HTTP status used by both response and body.
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Permanent compact semantic identity.
    pub const fn code(&self) -> &Code {
        &self.code
    }

    /// Identity for this specific occurrence.
    pub const fn occurrence(&self) -> &ProblemOccurrence {
        &self.occurrence
    }

    /// Reviewed typed public evidence before encoding.
    pub const fn evidence(&self) -> &E {
        &self.evidence
    }

    /// Encodes the strict canonical Problem profile.
    pub fn try_encode(&self) -> Result<EncodedProblem, ProblemEncodeError> {
        let evidence = serde_json::to_value(&self.evidence)
            .map_err(ProblemEncodeError::EvidenceSerialization)?;
        if !evidence.is_object() {
            return Err(ProblemEncodeError::EvidenceNotObject);
        }
        let instance = self.occurrence.instance().to_string();
        let wire = ProblemWire {
            type_uri: &self.type_uri,
            title: &self.title,
            status: self.status.as_u16(),
            detail: &self.detail,
            instance: &instance,
            code: &self.code,
            evidence: &evidence,
            suggestions: &self.suggestions,
        };
        let body = serde_json::to_vec(&wire).map_err(ProblemEncodeError::BodySerialization)?;
        let mut headers = self.headers.clone();
        headers.insert(CONTENT_TYPE, PROBLEM_JSON);
        Ok(EncodedProblem::new(self.status, headers, body))
    }
}

struct ProblemParts<'a, E: PublicEvidence> {
    definition: &'a CatalogDiagnostic,
    status: StatusCode,
    detail: Option<PublicText>,
    occurrence: ProblemOccurrence,
    evidence: E,
    headers: HeaderMap,
}

#[derive(Serialize)]
struct ProblemWire<'a> {
    #[serde(rename = "type")]
    type_uri: &'a str,
    title: &'a str,
    status: u16,
    detail: &'a str,
    instance: &'a str,
    code: &'a Code,
    evidence: &'a serde_json::Value,
    suggestions: &'a [String],
}

impl<C: CatalogSpec> Catalog<C> {
    /// Constructs a registered Problem whose policy input has a default.
    ///
    /// Header-aware policies cannot use this shortcut:
    ///
    /// ```compile_fail
    /// use recourse::{
    ///     catalog::{Catalog, CatalogSpec},
    ///     diagnostic::NoEvidence,
    ///     http::{HttpProblemType, ProblemOccurrence, Unauthorized},
    /// };
    ///
    /// fn missing_challenge<C, D>(catalog: &Catalog<C>, occurrence: ProblemOccurrence)
    /// where
    ///     C: CatalogSpec,
    ///     D: HttpProblemType<Catalog = C, Evidence = NoEvidence, Policy = Unauthorized>,
    /// {
    ///     let _ = catalog.try_problem::<D>(occurrence, NoEvidence);
    /// }
    /// ```
    pub fn try_problem<D>(
        &self,
        occurrence: ProblemOccurrence,
        evidence: D::Evidence,
    ) -> Result<Problem<D::Evidence>, ProblemBuildError>
    where
        D: HttpProblemType<Catalog = C>,
        D::Policy: HttpPolicy,
        <D::Policy as HttpPolicy>::Input: Default,
    {
        self.build_problem::<D>(occurrence, evidence, Default::default(), None)
    }

    /// Constructs a registered Problem with typed policy input.
    pub fn try_problem_with<D>(
        &self,
        occurrence: ProblemOccurrence,
        evidence: D::Evidence,
        input: <D::Policy as HttpPolicy>::Input,
    ) -> Result<Problem<D::Evidence>, ProblemBuildError>
    where
        D: HttpProblemType<Catalog = C>,
    {
        self.build_problem::<D>(occurrence, evidence, input, None)
    }

    /// Constructs a registered default-input Problem with reviewed dynamic detail.
    pub fn try_problem_with_detail<D>(
        &self,
        occurrence: ProblemOccurrence,
        evidence: D::Evidence,
        detail: PublicText,
    ) -> Result<Problem<D::Evidence>, ProblemBuildError>
    where
        D: HttpProblemType<Catalog = C>,
        <D::Policy as HttpPolicy>::Input: Default,
    {
        self.build_problem::<D>(occurrence, evidence, Default::default(), Some(detail))
    }

    fn build_problem<D>(
        &self,
        occurrence: ProblemOccurrence,
        evidence: D::Evidence,
        input: <D::Policy as HttpPolicy>::Input,
        detail: Option<PublicText>,
    ) -> Result<Problem<D::Evidence>, ProblemBuildError>
    where
        D: HttpProblemType<Catalog = C>,
    {
        let definition =
            self.problem_definition::<D>()
                .ok_or(ProblemBuildError::DiagnosticNotRegistered {
                    diagnostic: type_name::<D>(),
                })?;
        let status = StatusCode::from_u16(D::Policy::STATUS).map_err(|_| {
            ProblemBuildError::InvalidPolicyStatus {
                status: D::Policy::STATUS,
            }
        })?;
        let catalog_status =
            definition
                .http_status()
                .ok_or(ProblemBuildError::DiagnosticNotRegistered {
                    diagnostic: type_name::<D>(),
                })?;
        if catalog_status != status.as_u16() {
            return Err(ProblemBuildError::CatalogPolicyMismatch {
                code: definition.code().clone(),
                catalog_status,
                policy_status: status.as_u16(),
            });
        }
        let headers = D::Policy::headers(input).map_err(ProblemBuildError::Policy)?;
        for required in D::Policy::REQUIRED_HEADERS {
            if !headers.contains_key(*required) {
                return Err(ProblemBuildError::MissingPolicyHeader { name: required });
            }
        }
        Ok(Problem::from_parts(ProblemParts {
            definition,
            status,
            detail,
            occurrence,
            evidence,
            headers,
        }))
    }
}
