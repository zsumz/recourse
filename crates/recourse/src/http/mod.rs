//! Framework-neutral HTTP diagnostic surface declarations.

mod encoded;
mod occurrence;
mod policy;
mod problem;
mod uri_reference;

pub use encoded::EncodedProblem;
pub use occurrence::{
    CorrelationId, CorrelationIdError, MAX_CORRELATION_ID_BYTES, ProblemOccurrence,
    ProblemOccurrenceError,
};
pub(crate) use policy::mandatory_headers;
pub use policy::{
    AllowedMethods, AllowedMethodsError, BearerChallenge, BearerChallengeError, BearerUnauthorized,
    Fixed, HttpPolicy, HttpProblemType, MethodNotAllowed, PolicyError, RetryAfter, RetryAfterError,
    RetryAfterPolicy,
};
pub use problem::{Problem, ProblemBuildError, ProblemEncodeError};
pub use uri_reference::{UriReference, UriReferenceError};

#[cfg(test)]
mod encoded_test;
#[cfg(test)]
mod occurrence_test;
#[cfg(test)]
mod policy_header_test;
#[cfg(test)]
mod policy_test;
#[cfg(test)]
mod problem_contract_test;
#[cfg(test)]
mod problem_header_test;
#[cfg(test)]
mod problem_numeric_contract_test;
#[cfg(test)]
mod problem_test;
#[cfg(test)]
mod problem_uri_test;
