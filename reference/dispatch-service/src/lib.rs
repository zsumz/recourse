//! Framework-neutral Dispatch application behavior.

mod admission;
mod durable;
mod error;
mod fault;
mod generator;
mod queue;
mod registry;
mod service;

pub use admission::{
    AdmissionRefusal, DEFAULT_ADMISSION_CAPACITY, DEFAULT_RETRY_DELAY, JobAdmission,
};
pub use durable::{DispatchFailure, FailedDispatch};
pub use error::DispatchServiceError;
pub use fault::DispatchFault;
pub use generator::{JobIdGenerator, UlidJobIds};
pub use queue::QueueObservation;
pub use service::{CreateJobOutcome, DispatchService};

#[cfg(test)]
mod admission_test;
#[cfg(test)]
mod service_test;
