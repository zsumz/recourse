//! Public/private failure pair with public-only encoding behavior.

use crate::{
    diagnostic::PublicEvidence,
    http::{EncodedProblem, Problem, ProblemEncodeError},
};

use super::PrivateReport;

/// Sanitized public Problem paired with its operator-only private report.
#[derive(Debug)]
pub struct Fault<E: PublicEvidence> {
    problem: Problem<E>,
    report: PrivateReport,
}

impl<E: PublicEvidence> Fault<E> {
    /// Pairs already-constructed public and private failure values.
    pub const fn new(problem: Problem<E>, report: PrivateReport) -> Self {
        Self { problem, report }
    }

    /// Public caller-visible side.
    pub const fn problem(&self) -> &Problem<E> {
        &self.problem
    }

    /// Private operator-only side.
    pub const fn report(&self) -> &PrivateReport {
        &self.report
    }

    /// Encodes only the sanitized public side.
    pub fn try_encode(&self) -> Result<EncodedProblem, ProblemEncodeError> {
        self.problem.try_encode()
    }

    /// Splits the pair at an integration or reporting boundary.
    pub fn into_parts(self) -> (Problem<E>, PrivateReport) {
        (self.problem, self.report)
    }
}
