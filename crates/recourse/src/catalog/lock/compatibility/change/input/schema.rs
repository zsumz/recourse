//! Conservative schema compatibility diagnostics.

use crate::catalog::Code;

use super::{ChangeInput, CompatibilitySeverity};

impl ChangeInput {
    pub(crate) fn optional_property(code: &Code, path: &str) -> Self {
        Self::at(
            "REC-COMPAT-012",
            CompatibilitySeverity::Compatible,
            code,
            path,
        )
        .shapes("absent", "optional")
        .guidance(
            "Tolerant clients can ignore an optional field.",
            "Accept the field.",
        )
    }

    pub(crate) fn required_property(code: &Code, path: &str) -> Self {
        Self::at(
            "REC-COMPAT-013",
            CompatibilitySeverity::Breaking,
            code,
            path,
        )
        .shapes("absent", "required")
        .guidance(
            "Existing emitters may not provide the new field.",
            "Make it optional or mint a new code.",
        )
    }

    pub(crate) fn property_removed(code: &Code, path: &str) -> Self {
        Self::at(
            "REC-COMPAT-014",
            CompatibilitySeverity::Breaking,
            code,
            path,
        )
        .shapes("present", "absent")
        .guidance(
            "Clients may depend on this field.",
            "Restore it or mint a new code.",
        )
    }

    pub(crate) fn requiredness(code: &Code, path: &str, previous: bool, current: bool) -> Self {
        Self::at(
            "REC-COMPAT-015",
            CompatibilitySeverity::Breaking,
            code,
            path,
        )
        .shapes(requiredness(previous), requiredness(current))
        .guidance(
            "Changing requiredness changes producer or consumer obligations.",
            "Restore requiredness or mint a new code.",
        )
    }

    pub(crate) fn schema_changed(code: &Code, path: &str) -> Self {
        Self::at(
            "REC-COMPAT-016",
            CompatibilitySeverity::Breaking,
            code,
            path,
        )
        .shapes("accepted schema", "changed schema")
        .guidance(
            "The conservative profile cannot prove this change safe.",
            "Restore the shape or mint a new code.",
        )
    }
}

const fn requiredness(value: bool) -> &'static str {
    if value { "required" } else { "optional" }
}
