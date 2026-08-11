//! Schema-aware, resource-bounded health finding encoding.

use serde::Serialize;

use crate::{
    catalog::Code,
    diagnostic::PublicEvidence,
    materialize::{self, MaterializeError},
    wire::{BoundedJsonError, WireLimits, to_bounded_vec, validate_evidence, validate_wire_parts},
};

use super::{HealthEncodeError, HealthFinding, HealthFindingId, HealthSeverity, ObservationTime};

impl<E: PublicEvidence> HealthFinding<E> {
    /// Encodes the strict canonical health-finding profile.
    pub fn try_encode(&self) -> Result<Vec<u8>, HealthEncodeError> {
        to_bounded_vec(&self.try_wire()?, WireLimits::default()).map_err(map_json_error)
    }

    /// Builds the same canonical profile as a value for aggregate documents.
    ///
    /// Applications can compose this value directly into their health
    /// resource. Member order is not the canonical byte order.
    pub fn try_encode_value(&self) -> Result<serde_json::Value, HealthEncodeError> {
        let value =
            serde_json::to_value(self.try_wire()?).map_err(HealthEncodeError::BodySerialization)?;
        crate::wire::validate_value(&value, WireLimits::default())
            .map_err(HealthEncodeError::WireLimit)?;
        to_bounded_vec(&value, WireLimits::default()).map_err(map_json_error)?;
        Ok(value)
    }

    fn try_wire(&self) -> Result<HealthWire<'_>, HealthEncodeError> {
        let limits = WireLimits::default();
        let evidence = materialize::object(&self.evidence, limits).map_err(map_evidence_error)?;
        crate::catalog::validate_value(&self.evidence_validator, &evidence).map_err(
            |violation| HealthEncodeError::EvidenceSchemaMismatch {
                path: violation.path,
                reason: violation.reason,
            },
        )?;
        validate_evidence(&evidence, limits).map_err(HealthEncodeError::WireLimit)?;
        validate_wire_parts(
            9,
            &[
                self.id.as_str(),
                &self.type_uri,
                &self.title,
                &self.detail,
                self.observed_at.as_str(),
            ],
            &self.suggestions,
            limits,
        )
        .map_err(HealthEncodeError::WireLimit)?;
        Ok(HealthWire {
            id: &self.id,
            type_uri: &self.type_uri,
            code: &self.code,
            title: &self.title,
            detail: &self.detail,
            severity: self.severity,
            observed_at: &self.observed_at,
            evidence,
            suggestions: &self.suggestions,
        })
    }
}

fn map_evidence_error(error: MaterializeError) -> HealthEncodeError {
    match error {
        MaterializeError::Json(error) => HealthEncodeError::EvidenceSerialization(error),
        MaterializeError::NotObject => HealthEncodeError::EvidenceNotObject,
        MaterializeError::Limit(error) => HealthEncodeError::WireLimit(error),
    }
}

#[derive(Serialize)]
struct HealthWire<'a> {
    id: &'a HealthFindingId,
    #[serde(rename = "type")]
    type_uri: &'a str,
    code: &'a Code,
    title: &'a str,
    detail: &'a str,
    severity: HealthSeverity,
    observed_at: &'a ObservationTime,
    evidence: serde_json::Value,
    suggestions: &'a [String],
}

fn map_json_error(error: BoundedJsonError) -> HealthEncodeError {
    match error {
        BoundedJsonError::Serialize(error) => HealthEncodeError::BodySerialization(error),
        BoundedJsonError::Limit(error) => HealthEncodeError::WireLimit(error),
    }
}
