//! Append-only accepted, reserved, and retired diagnostic history.

mod accept;
mod compatibility;
mod entry;
mod error;
mod lifecycle;
mod parse;
mod replacement;

use std::io::Write;

use serde::{Deserialize, Serialize};

use super::CatalogArtifact;
pub use accept::AcceptanceMode;
pub use compatibility::{CompatibilityChange, CompatibilityReport, CompatibilitySeverity};
pub use entry::{LockEntry, LockState};
pub use error::{
    AcceptanceError, LockParseError, LockWriteError, ReservationError, RetirementError,
};
pub use lifecycle::Reservation;

/// Maximum accepted encoded size of a catalog lock.
pub const MAX_CATALOG_LOCK_BYTES: usize = 16 * 1024 * 1024;

/// Versioned append-only compatibility history for one catalog namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogLock {
    schema_version: u32,
    catalog: LockIdentity,
    entries: Vec<LockEntry>,
}

impl CatalogLock {
    /// Creates an initial lock accepting every current diagnostic definition.
    pub fn from_artifact(artifact: &CatalogArtifact) -> Self {
        Self {
            schema_version: 1,
            catalog: LockIdentity::from_artifact(artifact),
            entries: artifact
                .diagnostics()
                .iter()
                .cloned()
                .map(LockEntry::active)
                .collect(),
        }
    }

    /// Parses and semantically validates a bounded catalog lock.
    pub fn from_slice(body: &[u8]) -> Result<Self, LockParseError> {
        parse::parse_lock(body)
    }

    /// Lock format version.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Stable catalog name.
    pub fn name(&self) -> &str {
        &self.catalog.name
    }

    /// Stable catalog code prefix.
    pub fn prefix(&self) -> &str {
        &self.catalog.prefix
    }

    /// Permanent type-URI base.
    pub fn type_base(&self) -> &str {
        &self.catalog.type_base
    }

    /// Historical entries in strictly increasing numeric order.
    pub fn entries(&self) -> &[LockEntry] {
        &self.entries
    }

    /// Writes deterministic pretty JSON followed by one newline.
    pub fn write_pretty<W: Write>(&self, writer: W) -> Result<(), LockWriteError> {
        lifecycle::write_pretty(self, writer)
    }

    /// Reserves a never-before-used diagnostic identity.
    pub fn reserve(&mut self, reservation: Reservation) -> Result<&LockEntry, ReservationError> {
        lifecycle::reserve(self, reservation)
    }

    /// Classifies every compatibility-relevant difference from accepted history.
    pub fn check(&self, current: &CatalogArtifact) -> CompatibilityReport {
        compatibility::check(self, current)
    }

    /// Accepts current definitions when allowed by the selected acknowledgement mode.
    pub fn accept(
        &mut self,
        current: &CatalogArtifact,
        mode: AcceptanceMode,
    ) -> Result<CompatibilityReport, AcceptanceError> {
        accept::accept(self, current, mode)
    }

    /// Explicitly turns one active definition into a permanent tombstone.
    pub fn retire(
        &mut self,
        code: &super::Code,
        reason: impl Into<String>,
        replacement: Option<super::Code>,
    ) -> Result<&LockEntry, RetirementError> {
        accept::retire(self, code, reason.into(), replacement)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LockIdentity {
    pub(crate) name: String,
    pub(crate) prefix: String,
    pub(crate) type_base: String,
}

impl LockIdentity {
    fn from_artifact(artifact: &CatalogArtifact) -> Self {
        Self {
            name: artifact.name().to_owned(),
            prefix: artifact.prefix().to_owned(),
            type_base: artifact.type_base().to_owned(),
        }
    }
}
