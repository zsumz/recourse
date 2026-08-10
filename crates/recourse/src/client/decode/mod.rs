//! JSON parsing under explicit resource and shape budgets.

mod error;
mod limits;
mod parser;
mod validation;

pub use error::{DecodeError, DecodeLimit};
pub use limits::DecodeLimits;
pub(crate) use parser::decode_object;

#[cfg(test)]
mod decode_test;
