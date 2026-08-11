//! Schema-aware, resource-bounded Problem encoding.

use http::header::CONTENT_TYPE;
use serde::Serialize;

use crate::{
    catalog::Code,
    materialize::{self, MaterializeError},
    wire::{BoundedJsonError, WireLimits, to_bounded_vec, validate_evidence, validate_wire_parts},
};

use super::{EncodedProblem, PROBLEM_JSON, Problem, ProblemEncodeError, PublicEvidence};

impl<E: PublicEvidence> Problem<E> {
    /// Encodes the strict canonical Problem profile.
    pub fn try_encode(&self) -> Result<EncodedProblem, ProblemEncodeError> {
        let limits = WireLimits::default();
        let evidence = materialize::object(&self.evidence, limits).map_err(map_evidence_error)?;
        crate::catalog::validate_value(&self.evidence_validator, &evidence).map_err(
            |violation| ProblemEncodeError::EvidenceSchemaMismatch {
                path: violation.path,
                reason: violation.reason,
            },
        )?;
        let instance = self.occurrence.instance().to_string();
        validate_evidence(&evidence, limits).map_err(ProblemEncodeError::WireLimit)?;
        validate_wire_parts(
            8,
            &[&self.type_uri, &self.title, &self.detail, &instance],
            &self.suggestions,
            limits,
        )
        .map_err(ProblemEncodeError::WireLimit)?;
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
        let body = to_bounded_vec(&wire, limits).map_err(map_json_error)?;
        let mut headers = self.headers.clone();
        headers.insert(CONTENT_TYPE, PROBLEM_JSON);
        Ok(EncodedProblem::new(self.status, headers, body))
    }
}

fn map_evidence_error(error: MaterializeError) -> ProblemEncodeError {
    match error {
        MaterializeError::Json(error) => ProblemEncodeError::EvidenceSerialization(error),
        MaterializeError::NotObject => ProblemEncodeError::EvidenceNotObject,
        MaterializeError::Limit(error) => ProblemEncodeError::WireLimit(error),
    }
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

fn map_json_error(error: BoundedJsonError) -> ProblemEncodeError {
    match error {
        BoundedJsonError::Serialize(error) => ProblemEncodeError::BodySerialization(error),
        BoundedJsonError::Limit(error) => ProblemEncodeError::WireLimit(error),
    }
}
