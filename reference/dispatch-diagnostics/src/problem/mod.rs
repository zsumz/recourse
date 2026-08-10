//! Dispatch HTTP Problem marker declarations.

mod authentication_required;
mod idempotency_conflict;
mod internal_error;
mod job_not_found;
mod malformed_request;
mod service_temporarily_unavailable;
mod unsupported_media_type;
mod unsupported_method;
mod validation_failed;

pub use authentication_required::AuthenticationRequired;
pub use idempotency_conflict::IdempotencyConflict;
pub use internal_error::InternalError;
pub use job_not_found::JobNotFound;
pub use malformed_request::MalformedRequest;
pub use service_temporarily_unavailable::ServiceTemporarilyUnavailable;
pub use unsupported_media_type::UnsupportedMediaType;
pub use unsupported_method::UnsupportedMethod;
pub use validation_failed::ValidationFailed;
