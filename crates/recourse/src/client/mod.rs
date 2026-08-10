//! Bounded tolerant decoding of diagnostics received from remote systems.

mod decode;
mod issue;
mod member;
mod received;
mod received_health;
mod received_operation;
mod terminal;
mod typed;

pub(crate) use decode::decode_object;
pub use decode::{DecodeError, DecodeLimit, DecodeLimits};
pub use issue::ProtocolIssue;
pub use received::{Classification, ReceivedProblem};
pub use received_health::ReceivedHealthFinding;
pub use received_operation::ReceivedOperationDiagnostic;
pub use terminal::escape_terminal;
pub use typed::{ReceivedTypedProblem, TypedProblemError};

#[cfg(test)]
mod received_envelope_test;
#[cfg(test)]
mod received_test;
#[cfg(test)]
mod terminal_test;
