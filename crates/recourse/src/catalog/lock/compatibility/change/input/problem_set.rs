//! Governed API-operation Problem-set compatibility diagnostics.

use crate::catalog::Code;

use super::{ChangeInput, CompatibilitySeverity};

impl ChangeInput {
    pub(crate) fn problem_set_added(id: &str) -> Self {
        Self::new(
            "REC-COMPAT-018",
            CompatibilitySeverity::Compatible,
            None,
            &format!("problem_sets.{id}"),
        )
        .shapes("absent", "present")
        .guidance(
            "A new operation does not change existing operation contracts.",
            "Review and accept the new operation declaration.",
        )
    }

    pub(crate) fn problem_set_removed(id: &str) -> Self {
        Self::new(
            "REC-COMPAT-019",
            CompatibilitySeverity::Breaking,
            None,
            &format!("problem_sets.{id}"),
        )
        .shapes("present", "absent")
        .guidance(
            "Consumers may depend on the governed operation declaration.",
            "Restore the operation ID or acknowledge the break.",
        )
    }

    pub(crate) fn problem_set_member_added(id: &str, code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-020",
            CompatibilitySeverity::Breaking,
            code,
            &format!("problem_sets.{id}"),
        )
        .shapes("not emitted", "may be emitted")
        .guidance(
            "Existing operation consumers may not handle this Problem.",
            "Remove the Problem or acknowledge the break.",
        )
    }

    pub(crate) fn problem_set_member_removed(id: &str, code: &Code) -> Self {
        Self::at(
            "REC-COMPAT-021",
            CompatibilitySeverity::Compatible,
            code,
            &format!("problem_sets.{id}"),
        )
        .shapes("may be emitted", "not emitted")
        .guidance(
            "Emitting fewer Problems preserves existing consumer handling.",
            "Review and accept the narrower operation contract.",
        )
    }
}
