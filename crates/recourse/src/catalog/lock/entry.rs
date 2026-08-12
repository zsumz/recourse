//! One permanent diagnostic identity in the append-only lock history.

use serde::Serialize;

use crate::catalog::{CatalogDiagnostic, Code, CodeNumber};

/// Monotonic lifecycle state of one permanent diagnostic number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Number is allocated but has no accepted public definition.
    Reserved,
    /// Definition is present in the current catalog.
    Active,
    /// Definition is a permanent historical tombstone.
    Retired,
}

/// One reserved identity, accepted definition, or retired tombstone.
///
/// Lock entries are decoded only inside [`super::CatalogLock::from_slice`].
/// ```compile_fail
/// use recourse::catalog::LockEntry;
/// let _: Result<LockEntry, _> = serde_json::from_str("{}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum LockEntry {
    /// Allocated identity awaiting an accepted definition.
    Reserved {
        /// Permanent numeric identity.
        number: CodeNumber,
        /// Canonical compact code.
        code: Code,
        /// Permanent type URI.
        #[serde(rename = "type")]
        type_uri: String,
    },
    /// Current accepted diagnostic definition.
    Active {
        /// Complete compatibility-relevant public definition.
        diagnostic: CatalogDiagnostic,
    },
    /// Historical definition that can never be activated again.
    Retired {
        /// Last accepted public definition.
        diagnostic: CatalogDiagnostic,
        /// Human-authored reason for retirement.
        reason: String,
        /// Optional active or retired replacement identity.
        #[serde(skip_serializing_if = "Option::is_none")]
        replacement: Option<Code>,
    },
}

impl LockEntry {
    pub(crate) const fn active(diagnostic: CatalogDiagnostic) -> Self {
        Self::Active { diagnostic }
    }

    pub(crate) fn reserved(number: CodeNumber, code: Code, type_uri: String) -> Self {
        Self::Reserved {
            number,
            code,
            type_uri,
        }
    }

    /// Current monotonic lifecycle state.
    pub const fn state(&self) -> LockState {
        match self {
            Self::Reserved { .. } => LockState::Reserved,
            Self::Active { .. } => LockState::Active,
            Self::Retired { .. } => LockState::Retired,
        }
    }

    /// Permanent numeric identity.
    pub const fn number(&self) -> CodeNumber {
        match self {
            Self::Reserved { number, .. } => *number,
            Self::Active { diagnostic } | Self::Retired { diagnostic, .. } => diagnostic.number(),
        }
    }

    /// Canonical compact code.
    pub fn code(&self) -> &Code {
        match self {
            Self::Reserved { code, .. } => code,
            Self::Active { diagnostic } | Self::Retired { diagnostic, .. } => diagnostic.code(),
        }
    }

    /// Permanent type URI.
    pub fn type_uri(&self) -> &str {
        match self {
            Self::Reserved { type_uri, .. } => type_uri,
            Self::Active { diagnostic } | Self::Retired { diagnostic, .. } => diagnostic.type_uri(),
        }
    }

    /// Last accepted definition when active or retired.
    pub const fn diagnostic(&self) -> Option<&CatalogDiagnostic> {
        match self {
            Self::Reserved { .. } => None,
            Self::Active { diagnostic } | Self::Retired { diagnostic, .. } => Some(diagnostic),
        }
    }

    pub(crate) fn diagnostic_mut(&mut self) -> Option<&mut CatalogDiagnostic> {
        match self {
            Self::Reserved { .. } => None,
            Self::Active { diagnostic } | Self::Retired { diagnostic, .. } => Some(diagnostic),
        }
    }

    /// Explicit retirement rationale when this entry is a tombstone.
    pub fn retirement_reason(&self) -> Option<&str> {
        match self {
            Self::Retired { reason, .. } => Some(reason),
            Self::Reserved { .. } | Self::Active { .. } => None,
        }
    }

    /// Optional replacement for a retired identity.
    pub const fn replacement(&self) -> Option<&Code> {
        match self {
            Self::Retired { replacement, .. } => replacement.as_ref(),
            Self::Reserved { .. } | Self::Active { .. } => None,
        }
    }
}
