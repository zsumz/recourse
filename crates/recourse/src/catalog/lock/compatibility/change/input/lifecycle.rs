//! Identity and lifecycle compatibility diagnostics.

use crate::catalog::Code;

use super::{ChangeInput, CompatibilitySeverity};

impl ChangeInput {
    pub(crate) fn namespace(path: &str, previous: &str, current: &str) -> Self {
        Self::new(
            "REC-COMPAT-001",
            CompatibilitySeverity::Forbidden,
            None,
            path,
        )
        .shapes(previous, current)
        .guidance(
            "Catalog identity is permanent.",
            "Restore the accepted namespace.",
        )
    }

    pub(crate) fn retired_reused(code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-002",
            CompatibilitySeverity::Forbidden,
            code,
            "state",
        )
        .shapes("retired", "active")
        .guidance(
            "Retired codes remain tombstoned.",
            "Mint a new diagnostic code.",
        )
    }

    pub(crate) fn active_missing(code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-003",
            CompatibilitySeverity::Forbidden,
            code,
            "state",
        )
        .shapes("active", "absent")
        .guidance(
            "Deleting a declaration does not retire it.",
            "Retire it explicitly with a reason, or restore it.",
        )
    }

    pub(crate) fn diagnostic_added(code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-004",
            CompatibilitySeverity::Compatible,
            code,
            "state",
        )
        .shapes("absent", "active")
        .guidance(
            "Adding a new identity preserves existing contracts.",
            "Accept the definition.",
        )
    }

    pub(crate) fn reservation_activated(code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-005",
            CompatibilitySeverity::Compatible,
            code,
            "state",
        )
        .shapes("reserved", "active")
        .guidance(
            "The definition activates its matching reservation.",
            "Accept the definition.",
        )
    }
}
