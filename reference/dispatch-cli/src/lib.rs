//! Terminal-safe reference rendering for Dispatch diagnostics.

mod error;
mod field;
mod health;
mod operation;
mod problem;

pub use error::RenderError;
pub use health::render_health;
pub use operation::render_operation;
pub use problem::render_problem;
