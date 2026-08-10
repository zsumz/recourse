//! Shared structured evidence for caller-correctable validation failures.

mod evidence;
mod name;
mod pointer;

pub use evidence::{
    DEFAULT_MAX_VIOLATIONS, ValidationEvidence, ValidationEvidenceError, Violation,
    ViolationReason, ViolationSource,
};
pub use name::{HeaderName, HeaderNameError, ParameterName, ParameterNameError};
pub use pointer::{JsonPointer, JsonPointerError};

#[cfg(test)]
mod evidence_test;
#[cfg(test)]
mod name_test;
#[cfg(test)]
mod pointer_test;
