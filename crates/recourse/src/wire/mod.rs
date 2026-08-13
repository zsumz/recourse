//! Shared resource profile for emitted and received diagnostic JSON.

mod error;
mod limits;
mod validation;
mod writer;

pub use error::{WireLimit, WireLimitError};
pub use limits::WireLimits;
pub(crate) use validation::{
    validate_embedded, validate_evidence, validate_value, validate_wire_parts,
};
pub(crate) use writer::{BoundedJsonError, to_bounded_vec};

#[cfg(test)]
mod error_test;
