//! Strict construction and canonical encoding for current health findings.

use std::any::type_name;

use serde::Serialize;

use crate::{
    catalog::{Catalog, CatalogSpec, Code},
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

    /// Encodes the strict canonical health-finding profile.
    pub fn try_encode(&self) -> Result<Vec<u8>, HealthEncodeError> {
        serde_json::to_vec(&self.try_wire()?).map_err(HealthEncodeError::BodySerialization)
    }

    /// Builds the same canonical profile as a value for aggregate documents.
    ///
    /// Applications that publish a finding inside their own resource compose
    /// this value directly instead of decoding [`HealthFinding::try_encode`].
    ///
    /// The value holds the same members and values as
    /// [`HealthFinding::try_encode`], but member *order* is the order
    /// `serde_json::Value` keeps (alphabetical), not the canonical byte order.
    /// Serialize this value yourself only when byte-canonical order is not
    /// required; use [`HealthFinding::try_encode`] for canonical bytes.
    pub fn try_encode_value(&self) -> Result<serde_json::Value, HealthEncodeError> {
        serde_json::to_value(self.try_wire()?).map_err(HealthEncodeError::BodySerialization)
    }

    /// Validates evidence once for both canonical representations.
    fn try_wire(&self) -> Result<HealthWire<'_>, HealthEncodeError> {
        let evidence = serde_json::to_value(&self.evidence)
            .map_err(HealthEncodeError::EvidenceSerialization)?;
        if !evidence.is_object() {
            return Err(HealthEncodeError::EvidenceNotObject);
        }
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
        })
    }
}
