//! Schema-aware, resource-bounded durable diagnostic encoding.

use serde::Serialize;

use crate::{
    catalog::Code,
    diagnostic::PublicEvidence,
    materialize::{self, MaterializeError},
    wire::{
        BoundedJsonError, WireLimits, to_bounded_vec, validate_embedded, validate_evidence,
        validate_wire_parts,
    },
};

use super::{OperationDiagnostic, OperationDiagnosticId, OperationEncodeError};

impl<E: PublicEvidence, I: PublicEvidence> OperationDiagnostic<E, I> {
    /// Encodes the strict canonical durable-diagnostic profile.
    pub fn try_encode(&self) -> Result<Vec<u8>, OperationEncodeError> {
        to_bounded_vec(&self.try_wire()?, WireLimits::default()).map_err(map_json_error)
    }

    /// Builds the same canonical profile as a value for durable records.
    ///
    /// Applications that store or embed a diagnostic compose this value
    /// directly instead of decoding [`OperationDiagnostic::try_encode`].
    /// Member order in the returned value is not the canonical byte order.
    pub fn try_encode_value(&self) -> Result<serde_json::Value, OperationEncodeError> {
        let value = serde_json::to_value(self.try_wire()?)
            .map_err(OperationEncodeError::BodySerialization)?;
        crate::wire::validate_value(&value, WireLimits::default())
            .map_err(OperationEncodeError::WireLimit)?;
        to_bounded_vec(&value, WireLimits::default()).map_err(map_json_error)?;
        Ok(value)
    }

    fn try_wire(&self) -> Result<OperationWire<'_>, OperationEncodeError> {
        let limits = WireLimits::default();
        let evidence = materialize::object(&self.evidence, limits).map_err(map_evidence_error)?;
        crate::catalog::validate_value(&self.evidence_validator, &evidence).map_err(
            |violation| OperationEncodeError::EvidenceSchemaMismatch {
                path: violation.path,
                reason: violation.reason,
            },
        )?;
        let impact = materialize::object(&self.impact, limits).map_err(map_impact_error)?;
        crate::catalog::validate_value(&self.impact_validator, &impact).map_err(|violation| {
            OperationEncodeError::ImpactSchemaMismatch {
                path: violation.path,
                reason: violation.reason,
            }
        })?;
        validate_evidence(&evidence, limits).map_err(OperationEncodeError::WireLimit)?;
        validate_embedded(&impact, limits).map_err(OperationEncodeError::WireLimit)?;
        validate_wire_parts(
            8,
            &[self.id.as_str(), &self.type_uri, &self.title, &self.detail],
            &self.suggestions,
            limits,
        )
        .map_err(OperationEncodeError::WireLimit)?;
        Ok(OperationWire {
            id: &self.id,
            type_uri: &self.type_uri,
            code: &self.code,
            title: &self.title,
            detail: &self.detail,
            evidence,
            impact,
            suggestions: &self.suggestions,
        })
    }
}

fn map_evidence_error(error: MaterializeError) -> OperationEncodeError {
    match error {
        MaterializeError::Json(error) => OperationEncodeError::EvidenceSerialization(error),
        MaterializeError::NotObject => OperationEncodeError::EvidenceNotObject,
        MaterializeError::Limit(error) => OperationEncodeError::WireLimit(error),
    }
}

fn map_impact_error(error: MaterializeError) -> OperationEncodeError {
    match error {
        MaterializeError::Json(error) => OperationEncodeError::ImpactSerialization(error),
        MaterializeError::NotObject => OperationEncodeError::ImpactNotObject,
        MaterializeError::Limit(error) => OperationEncodeError::WireLimit(error),
    }
}

#[derive(Serialize)]
struct OperationWire<'a> {
    id: &'a OperationDiagnosticId,
    #[serde(rename = "type")]
    type_uri: &'a str,
    code: &'a Code,
    title: &'a str,
    detail: &'a str,
    evidence: serde_json::Value,
    impact: serde_json::Value,
    suggestions: &'a [String],
}

fn map_json_error(error: BoundedJsonError) -> OperationEncodeError {
    match error {
        BoundedJsonError::Serialize(error) => OperationEncodeError::BodySerialization(error),
        BoundedJsonError::Limit(error) => OperationEncodeError::WireLimit(error),
    }
}
