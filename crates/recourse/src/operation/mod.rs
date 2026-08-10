//! Durable diagnostic declarations and envelopes for accepted work.

mod declaration;
mod diagnostic;
mod error;
mod id;

pub use declaration::OperationDiagnosticType;
pub use diagnostic::OperationDiagnostic;
pub use error::{OperationBuildError, OperationEncodeError};
pub use id::{
    MAX_OPERATION_DIAGNOSTIC_ID_BYTES, OperationDiagnosticId, OperationDiagnosticIdError,
};

#[cfg(test)]
mod diagnostic_test;
#[cfg(test)]
mod id_test;
