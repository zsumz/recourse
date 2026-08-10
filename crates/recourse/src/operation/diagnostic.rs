//! Strict construction and canonical encoding for accepted-work failures.

use std::any::type_name;

use serde::Serialize;

use crate::{
    catalog::{Catalog, CatalogSpec, Code},
    diagnostic::PublicEvidence,
};

use super::{
    OperationBuildError, OperationDiagnosticId, OperationDiagnosticType, OperationEncodeError,
};

/// Typed durable operation failure awaiting persistence or transport.
#[derive(Debug)]
pub struct OperationDiagnostic<E: PublicEvidence, I: PublicEvidence> {
    id: OperationDiagnosticId,
    type_uri: String,
    code: Code,
    title: String,
    detail: String,
    evidence: E,
    impact: I,
    suggestions: Vec<String>,
}

impl<E: PublicEvidence, I: PublicEvidence> OperationDiagnostic<E, I> {
    /// Stable identifier for this durable diagnostic occurrence.
    pub const fn id(&self) -> &OperationDiagnosticId {
        &self.id
    }

    /// Permanent semantic diagnostic code.
    pub const fn code(&self) -> &Code {
        &self.code
    }

    /// Reviewed typed public evidence.
    pub const fn evidence(&self) -> &E {
        &self.evidence
    }

    /// Reviewed typed public impact facts.
    pub const fn impact(&self) -> &I {
        &self.impact
    }

    /// Encodes the strict canonical durable-diagnostic profile.
    pub fn try_encode(&self) -> Result<Vec<u8>, OperationEncodeError> {
        serde_json::to_vec(&self.try_wire()?).map_err(OperationEncodeError::BodySerialization)
    }

    /// Builds the same canonical profile as a value for durable records.
    ///
    /// Applications that store or embed a diagnostic compose this value
    /// directly instead of decoding [`OperationDiagnostic::try_encode`].
    ///
    /// The value holds the same members and values as
    /// [`OperationDiagnostic::try_encode`], but member *order* is the order
    /// `serde_json::Value` keeps (alphabetical), not the canonical byte order.
    /// Serialize this value yourself only when byte-canonical order is not
    /// required; use [`OperationDiagnostic::try_encode`] for canonical bytes.
    pub fn try_encode_value(&self) -> Result<serde_json::Value, OperationEncodeError> {
        serde_json::to_value(self.try_wire()?).map_err(OperationEncodeError::BodySerialization)
    }

    /// Validates evidence and impact once for both canonical representations.
    fn try_wire(&self) -> Result<OperationWire<'_>, OperationEncodeError> {
        let evidence = serde_json::to_value(&self.evidence)
            .map_err(OperationEncodeError::EvidenceSerialization)?;
        if !evidence.is_object() {
            return Err(OperationEncodeError::EvidenceNotObject);
        }
        let impact = serde_json::to_value(&self.impact)
            .map_err(OperationEncodeError::ImpactSerialization)?;
        if !impact.is_object() {
            return Err(OperationEncodeError::ImpactNotObject);
        }
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

impl<C: CatalogSpec> Catalog<C> {
    /// Constructs a diagnostic registered on the durable-operation surface.
    ///
    /// Impact is a second typed public value, separate from evidence, and both
    /// are required. There is no HTTP status: the request already succeeded.
    ///
    /// ```
    /// use recourse::{
    ///     catalog::{Catalog, CatalogSpec, CodeNumber},
    ///     diagnostic::{DiagnosticType, PublicEvidence},
    ///     operation::{OperationDiagnosticId, OperationDiagnosticType},
    /// };
    /// use schemars::JsonSchema;
    /// use serde::Serialize;
    ///
    /// # enum ServiceCatalog {}
    /// # impl CatalogSpec for ServiceCatalog {
    /// #     const NAME: &'static str = "example-service";
    /// #     const PREFIX: &'static str = "EXM";
    /// #     const TYPE_BASE: &'static str = "https://example.invalid/problems/";
    /// # }
    /// #[derive(Debug, Serialize, JsonSchema)]
    /// struct DispatchFailedEvidence {
    ///     job_id: String,
    ///     attempt: u32,
    /// }
    ///
    /// #[derive(Debug, Serialize, JsonSchema)]
    /// struct DispatchImpact {
    ///     destination_changed: bool,
    ///     created_artifacts: u32,
    /// }
    ///
    /// impl PublicEvidence for DispatchFailedEvidence {}
    /// impl PublicEvidence for DispatchImpact {}
    ///
    /// enum DispatchFailed {}
    ///
    /// impl DiagnosticType for DispatchFailed {
    ///     type Catalog = ServiceCatalog;
    ///     type Evidence = DispatchFailedEvidence;
    ///
    ///     const NUMBER: CodeNumber = CodeNumber::new(1009);
    ///     const TITLE: &'static str = "Job dispatch failed";
    ///     const DETAIL: &'static str = "The job was accepted but could not be dispatched.";
    ///     const SUGGESTIONS: &'static [&'static str] = &["Inspect the failed attempt."];
    ///     const DOCS: &'static str = "Retry after correcting the destination.";
    /// }
    ///
    /// impl OperationDiagnosticType for DispatchFailed {
    ///     type Impact = DispatchImpact;
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let catalog = Catalog::<ServiceCatalog>::builder()
    ///     .operation::<DispatchFailed>()
    ///     .build()?;
    /// let diagnostic = catalog.try_operation::<DispatchFailed>(
    ///     OperationDiagnosticId::try_new("dia_01K00000000000000000000000-3")?,
    ///     DispatchFailedEvidence {
    ///         job_id: "job_01K00000000000000000000000".to_owned(),
    ///         attempt: 3,
    ///     },
    ///     DispatchImpact {
    ///         destination_changed: false,
    ///         created_artifacts: 2,
    ///     },
    /// )?;
    ///
    /// assert_eq!(diagnostic.code().to_string(), "EXM-1009");
    /// let document = diagnostic.try_encode_value()?;
    /// assert_eq!(document["impact"]["created_artifacts"], 2);
    /// # Ok(())
    /// # }
    /// # assert!(example().is_ok());
    /// ```
    pub fn try_operation<D>(
        &self,
        id: OperationDiagnosticId,
        evidence: D::Evidence,
        impact: D::Impact,
    ) -> Result<OperationDiagnostic<D::Evidence, D::Impact>, OperationBuildError>
    where
        D: OperationDiagnosticType<Catalog = C>,
    {
        let definition = self.operation_definition::<D>().ok_or(
            OperationBuildError::DiagnosticNotRegistered {
                diagnostic: type_name::<D>(),
            },
        )?;
        Ok(OperationDiagnostic {
            id,
            type_uri: definition.type_uri().to_owned(),
            code: definition.code().clone(),
            title: definition.title().to_owned(),
            detail: definition.detail().to_owned(),
            evidence,
            impact,
            suggestions: definition.suggestions().to_vec(),
        })
    }
}
