//! Structured stable compatibility diagnostics and report summary.

mod input;

use crate::catalog::Code;
use serde::Serialize;

pub(crate) use input::ChangeInput;

/// Severity and acceptance policy for one catalog difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilitySeverity {
    /// Safe for existing protocol consumers.
    Compatible,
    /// Requires explicit acknowledgement before acceptance.
    Breaking,
    /// Violates permanent history and can never be accepted.
    Forbidden,
}

/// One precise compatibility-relevant difference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompatibilityChange {
    id: &'static str,
    severity: CompatibilitySeverity,
    code: Option<Code>,
    path: String,
    previous: String,
    current: String,
    reason: String,
    action: String,
}

impl CompatibilityChange {
    /// Stable Recourse tooling diagnostic ID.
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Compatibility classification.
    pub const fn severity(&self) -> CompatibilitySeverity {
        self.severity
    }

    /// Affected catalog diagnostic, or none for namespace changes.
    pub const fn code(&self) -> Option<&Code> {
        self.code.as_ref()
    }

    /// Precise compatibility path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Previous accepted shape summary.
    pub fn previous(&self) -> &str {
        &self.previous
    }

    /// Current proposed shape summary.
    pub fn current(&self) -> &str {
        &self.current
    }

    /// Why existing consumers may care.
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// Corrective or acknowledgement action.
    pub fn action(&self) -> &str {
        &self.action
    }
}

/// Deterministically ordered compatibility analysis result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompatibilityReport {
    changes: Vec<CompatibilityChange>,
}

impl CompatibilityReport {
    pub(crate) fn new(mut changes: Vec<CompatibilityChange>) -> Self {
        changes.sort_by(|left, right| {
            let left_key = (left.code.as_ref(), left.path.as_str(), left.id);
            let right_key = (right.code.as_ref(), right.path.as_str(), right.id);
            left_key.cmp(&right_key)
        });
        Self { changes }
    }

    /// Every classified difference in deterministic order.
    pub fn changes(&self) -> &[CompatibilityChange] {
        &self.changes
    }

    /// Whether no breaking or forbidden difference exists.
    pub fn is_compatible(&self) -> bool {
        self.changes
            .iter()
            .all(|change| change.severity == CompatibilitySeverity::Compatible)
    }

    /// Whether at least one difference requires acknowledgement.
    pub fn has_breaking(&self) -> bool {
        self.changes
            .iter()
            .any(|change| change.severity == CompatibilitySeverity::Breaking)
    }

    /// Whether permanent history would be violated.
    pub fn has_forbidden(&self) -> bool {
        self.changes
            .iter()
            .any(|change| change.severity == CompatibilitySeverity::Forbidden)
    }
}

pub(crate) fn push(changes: &mut Vec<CompatibilityChange>, input: ChangeInput) {
    changes.push(input.finish());
}
