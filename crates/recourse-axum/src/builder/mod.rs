//! Fail-closed configuration of the catalog-typed Axum layer.

mod config;
mod error;
mod faults;
mod internal;

pub use config::RecourseLayerBuilder;
pub use error::LayerBuildError;
