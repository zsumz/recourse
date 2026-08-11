//! Strict construction and canonical encoding for accepted-work failures.

mod encode;

use std::{any::type_name, sync::Arc};

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
    evidence_validator: Arc<jsonschema::Validator>,
    impact_validator: Arc<jsonschema::Validator>,
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
        let validators = self.validators(definition.number()).ok_or_else(|| {
            OperationBuildError::ValidatorsMissing {
                code: definition.code().clone(),
            }
        })?;
        let Some(impact_validator) = validators.impact() else {
            return Err(OperationBuildError::ValidatorsMissing {
                code: definition.code().clone(),
            });
        };
        Ok(OperationDiagnostic {
            id,
            type_uri: definition.type_uri().to_owned(),
            code: definition.code().clone(),
            title: definition.title().to_owned(),
            detail: definition.detail().to_owned(),
            evidence,
            impact,
            suggestions: definition.suggestions().to_vec(),
            evidence_validator: validators.evidence(),
            impact_validator,
        })
    }
}
