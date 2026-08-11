//! Strict typed Problem construction and canonical JSON encoding.

mod encode;
mod error;

use std::{any::type_name, sync::Arc};

use http::{HeaderMap, HeaderValue, StatusCode};

use crate::{
    catalog::{Catalog, CatalogDiagnostic, CatalogSpec, Code, DiagnosticValidators},
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
    evidence_validator: Arc<jsonschema::Validator>,
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
            evidence_validator: parts.evidence_validator,
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
}

struct ProblemParts<'a, E: PublicEvidence> {
    definition: &'a CatalogDiagnostic,
    status: StatusCode,
    detail: Option<PublicText>,
    occurrence: ProblemOccurrence,
    evidence: E,
    headers: HeaderMap,
    evidence_validator: Arc<jsonschema::Validator>,
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
    ///     http::{BearerUnauthorized, HttpProblemType, ProblemOccurrence},
    /// };
    ///
    /// fn missing_challenge<C, D>(catalog: &Catalog<C>, occurrence: ProblemOccurrence)
    /// where
    ///     C: CatalogSpec,
    ///     D: HttpProblemType<Catalog = C, Evidence = NoEvidence, Policy = BearerUnauthorized>,
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
        for required in super::mandatory_headers(status.as_u16())
            .iter()
            .chain(D::Policy::REQUIRED_HEADERS)
        {
            if !headers.contains_key(*required) {
                return Err(ProblemBuildError::MissingPolicyHeader { name: required });
            }
        }
        let evidence_validator = self
            .validators(definition.number())
            .map(DiagnosticValidators::evidence)
            .ok_or_else(|| ProblemBuildError::ValidatorMissing {
                code: definition.code().clone(),
            })?;
        Ok(Problem::from_parts(ProblemParts {
            definition,
            status,
            detail,
            occurrence,
            evidence,
            headers,
            evidence_validator,
        }))
    }
}
