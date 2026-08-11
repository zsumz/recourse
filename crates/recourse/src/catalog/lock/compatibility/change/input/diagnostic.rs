//! Metadata and envelope-surface compatibility diagnostics.

use crate::catalog::Code;

use super::{ChangeInput, CompatibilitySeverity};

impl ChangeInput {
    pub(crate) fn title(code: &Code, previous: &str, current: &str) -> Self {
        Self::at(
            "REC-COMPAT-006",
            CompatibilitySeverity::Breaking,
            code,
            "title",
        )
        .shapes(previous, current)
        .guidance(
            "Clients may display or classify by the stable title.",
            "Restore the title or acknowledge the break.",
        )
    }

    pub(crate) fn guidance_changed(code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-007",
            CompatibilitySeverity::Compatible,
            code,
            "guidance",
        )
        .shapes("accepted guidance", "changed guidance")
        .guidance(
            "Caller guidance is not machine-readable protocol behavior.",
            "Review and accept the guidance change.",
        )
    }

    pub(crate) fn surface_added(code: &Code, surface: &str) -> Self {
        let path = format!("surfaces.{surface}");
        Self::at(
            "REC-COMPAT-008",
            CompatibilitySeverity::Compatible,
            code,
            &path,
        )
        .shapes("absent", "present")
        .guidance(
            "Adding an envelope surface preserves existing ones.",
            "Accept the surface.",
        )
    }

    pub(crate) fn surface_removed(code: &Code, surface: &str) -> Self {
        let path = format!("surfaces.{surface}");
        Self::at(
            "REC-COMPAT-009",
            CompatibilitySeverity::Breaking,
            code,
            &path,
        )
        .shapes("present", "absent")
        .guidance(
            "Consumers may depend on this envelope surface.",
            "Restore it or acknowledge the break.",
        )
    }

    pub(crate) fn http_status(code: &Code, previous: u16, current: u16) -> Self {
        Self::at(
            "REC-COMPAT-010",
            CompatibilitySeverity::Breaking,
            code,
            "surfaces.http.status",
        )
        .shapes(&previous.to_string(), &current.to_string())
        .guidance(
            "Clients may branch on HTTP status.",
            "Restore the status or mint a new code.",
        )
    }

    pub(crate) fn http_contract(code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-011",
            CompatibilitySeverity::Breaking,
            code,
            "surfaces.http",
        )
        .shapes("accepted policy and headers", "changed policy or headers")
        .guidance(
            "Clients may depend on required response headers.",
            "Restore the HTTP contract or acknowledge the break.",
        )
    }
}
