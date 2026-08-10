//! Axum lifecycle translation around framework-neutral Recourse values.

#![deny(missing_docs)]

mod builder;
mod context;
mod failure;
mod layer;
mod observation;
mod panic;
mod request_id;
mod runtime;
mod scope;

pub use builder::{LayerBuildError, RecourseLayerBuilder};
pub use context::{MissingProblemContext, ProblemContext};
pub use failure::{HandlerResult, HttpFailure};
pub use layer::{RecourseLayer, RecourseService};
pub use request_id::{RequestIdGenerator, UlidRequestIds};

#[cfg(test)]
mod builder_test;
#[cfg(test)]
mod fallback_test;
#[cfg(test)]
mod fault_test;
#[cfg(test)]
mod layer_test;
#[cfg(test)]
mod request_id_test;
#[cfg(test)]
mod service_error_test;
