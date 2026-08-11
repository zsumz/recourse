//! JSON parsing under explicit resource and shape budgets.

mod error;
mod parser;
mod validation;

pub use crate::wire::WireLimits as DecodeLimits;
pub use error::{DecodeError, DecodeLimit};
pub(crate) use parser::decode_object;

#[cfg(test)]
mod decode_test;
