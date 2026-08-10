//! Stable compatibility diagnostic construction vocabulary.

mod diagnostic;
mod lifecycle;
mod schema;

use crate::catalog::Code;

use super::{CompatibilityChange, CompatibilitySeverity};

pub(crate) struct ChangeInput {
    id: &'static str,
    severity: CompatibilitySeverity,
    code: Option<Code>,
    path: String,
    previous: String,
    current: String,
    reason: String,
    action: String,
}

impl ChangeInput {
    pub(super) fn finish(self) -> CompatibilityChange {
        CompatibilityChange {
            id: self.id,
            severity: self.severity,
            code: self.code,
            path: self.path,
            previous: self.previous,
            current: self.current,
            reason: self.reason,
            action: self.action,
        }
    }

    fn at(id: &'static str, severity: CompatibilitySeverity, code: &Code, path: &str) -> Self {
        Self::new(id, severity, Some(code.clone()), path)
    }

    fn new(
        id: &'static str,
        severity: CompatibilitySeverity,
        code: Option<Code>,
        path: &str,
    ) -> Self {
        Self {
            id,
            severity,
            code,
            path: path.to_owned(),
            previous: String::new(),
            current: String::new(),
            reason: String::new(),
            action: String::new(),
        }
    }

    fn shapes(mut self, previous: &str, current: &str) -> Self {
        previous.clone_into(&mut self.previous);
        current.clone_into(&mut self.current);
        self
    }

    fn guidance(mut self, reason: &str, action: &str) -> Self {
        reason.clone_into(&mut self.reason);
        action.clone_into(&mut self.action);
        self
    }
}
