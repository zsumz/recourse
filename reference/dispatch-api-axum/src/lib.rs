//! Thin Axum translation for the framework-neutral Dispatch reference service.

mod auth;
mod error;
mod fault;
mod health;
mod input;
mod jobs;
mod method;
mod router;
mod state;

pub use error::ApiBuildError;
pub use fault::FaultLog;
pub use router::{router, router_with_admission};
