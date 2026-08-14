//! Built-in HTTP authentication challenge policies.

mod basic;
mod bearer;
mod realm;

pub use basic::{BasicChallenge, BasicChallengeError, BasicUnauthorized};
pub use bearer::{BearerChallenge, BearerChallengeError, BearerUnauthorized};
