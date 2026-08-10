//! Structural pairing of a public Problem and private diagnostic context.

mod pair;
mod report;

pub use pair::Fault;
pub use report::{PrivateContext, PrivateReport};

#[cfg(test)]
mod fault_test;
#[cfg(test)]
mod report_test;
