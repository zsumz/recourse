//! Marker boundary for diagnostics usable as health findings.

use crate::diagnostic::DiagnosticType;

/// Declares a diagnostic usable as a current health finding.
pub trait HealthFindingType: DiagnosticType {}
