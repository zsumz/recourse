//! Strict construction and canonical encoding for current health findings.

mod encode;

use std::{any::type_name, sync::Arc};

use crate::{
    catalog::{Catalog, CatalogSpec, Code, DiagnosticValidators},
    diagnostic::PublicEvidence,
};

use super::{
    HealthBuildError, HealthEncodeError, HealthFindingId, HealthFindingType, HealthSeverity,
    ObservationTime,
};

/// Typed current service-state finding awaiting resource encoding.
#[derive(Debug)]
pub struct HealthFinding<E: PublicEvidence> {
    id: HealthFindingId,
    type_uri: String,
    code: Code,
    title: String,
    detail: String,
    severity: HealthSeverity,
    observed_at: ObservationTime,
    evidence: E,
    suggestions: Vec<String>,
    evidence_validator: Arc<jsonschema::Validator>,
}

impl<E: PublicEvidence> HealthFinding<E> {
    /// Stable identifier for this observed finding.
    pub const fn id(&self) -> &HealthFindingId {
        &self.id
    }

    /// Permanent semantic diagnostic code.
    pub const fn code(&self) -> &Code {
        &self.code
    }

    /// Current severity of the finding.
    pub const fn severity(&self) -> HealthSeverity {
        self.severity
    }

    /// Canonical observation time.
    pub const fn observed_at(&self) -> &ObservationTime {
        &self.observed_at
    }

    /// Reviewed typed public evidence.
    pub const fn evidence(&self) -> &E {
        &self.evidence
    }
}

impl<C: CatalogSpec> Catalog<C> {
    /// Constructs a finding registered on the current-health surface.
    ///
    /// A finding is resource data describing present state, so it carries a
    /// severity and an observation time instead of a status. A healthy service
    /// publishes no finding at all; the surrounding resource says so.
    ///
    /// ```
    /// use recourse::{
    ///     catalog::{Catalog, CatalogSpec, CodeNumber},
    ///     diagnostic::{DiagnosticType, PublicEvidence},
    ///     health::{HealthFindingId, HealthFindingType, HealthSeverity, ObservationTime},
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
    /// struct QueueUnavailableEvidence {
    ///     consecutive_failures: u32,
    /// }
    ///
    /// impl PublicEvidence for QueueUnavailableEvidence {}
    ///
    /// enum QueueUnavailable {}
    ///
    /// impl DiagnosticType for QueueUnavailable {
    ///     type Catalog = ServiceCatalog;
    ///     type Evidence = QueueUnavailableEvidence;
    ///
    ///     const NUMBER: CodeNumber = CodeNumber::new(1010);
    ///     const TITLE: &'static str = "Job queue unavailable";
    ///     const DETAIL: &'static str = "The worker cannot currently reach the job queue.";
    ///     const SUGGESTIONS: &'static [&'static str] = &["Check queue connectivity."];
    ///     const DOCS: &'static str = "Verify credentials and network policy.";
    /// }
    ///
    /// impl HealthFindingType for QueueUnavailable {}
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let catalog = Catalog::<ServiceCatalog>::builder()
    ///     .health::<QueueUnavailable>()
    ///     .build()?;
    /// let finding = catalog.try_health::<QueueUnavailable>(
    ///     HealthFindingId::try_new("finding_queue-unavailable")?,
    ///     HealthSeverity::Degraded,
    ///     ObservationTime::parse("2026-08-10T14:31:00Z")?,
    ///     QueueUnavailableEvidence {
    ///         consecutive_failures: 3,
    ///     },
    /// )?;
    ///
    /// // Compose the value into an application-owned health resource.
    /// let document = finding.try_encode_value()?;
    /// assert_eq!(document["severity"], "degraded");
    /// assert_eq!(document["observed_at"], "2026-08-10T14:31:00Z");
    /// # Ok(())
    /// # }
    /// # assert!(example().is_ok());
    /// ```
    pub fn try_health<D>(
        &self,
        id: HealthFindingId,
        severity: HealthSeverity,
        observed_at: ObservationTime,
        evidence: D::Evidence,
    ) -> Result<HealthFinding<D::Evidence>, HealthBuildError>
    where
        D: HealthFindingType<Catalog = C>,
    {
        let definition =
            self.health_definition::<D>()
                .ok_or(HealthBuildError::DiagnosticNotRegistered {
                    diagnostic: type_name::<D>(),
                })?;
        let evidence_validator = self
            .validators(definition.number())
            .map(DiagnosticValidators::evidence)
            .ok_or_else(|| HealthBuildError::ValidatorMissing {
                code: definition.code().clone(),
            })?;
        Ok(HealthFinding {
            id,
            type_uri: definition.type_uri().to_owned(),
            code: definition.code().clone(),
            title: definition.title().to_owned(),
            detail: definition.detail().to_owned(),
            severity,
            observed_at,
            evidence,
            suggestions: definition.suggestions().to_vec(),
            evidence_validator,
        })
    }
}
