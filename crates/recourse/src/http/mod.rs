//! Framework-neutral HTTP diagnostic surface declarations.

mod encoded;
mod occurrence;
mod policy;
mod problem;

pub use encoded::EncodedProblem;
pub use occurrence::{
    CorrelationId, CorrelationIdError, MAX_CORRELATION_ID_BYTES, ProblemOccurrence,
    ProblemOccurrenceError,
};
pub use policy::{
    AllowedMethods, AllowedMethodsError, BearerChallenge, BearerChallengeError, Fixed, HttpPolicy,
    HttpProblemType, MethodNotAllowed, PolicyError, RetryAfter, RetryAfterPolicy, Unauthorized,
};
pub use problem::{Problem, ProblemBuildError, ProblemEncodeError};

#[cfg(test)]
mod encoded_test;
#[cfg(test)]
mod occurrence_test;
#[cfg(test)]
mod policy_header_test;
#[cfg(test)]
mod policy_test;
#[cfg(test)]
mod problem_header_test;
#[cfg(test)]
mod problem_test;
