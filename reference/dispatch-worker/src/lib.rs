//! Framework-neutral processing for accepted Dispatch jobs.

mod error;
mod health;
mod record;
mod worker;

pub use error::DispatchWorkerError;
pub use health::PublishedQueueHealth;
pub use record::{RecordFailureOutcome, RecordedDispatchFailure};
pub use worker::DispatchWorker;
