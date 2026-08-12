//! Stable human-readable catalog issue descriptions.

use std::fmt::{self, Display, Formatter};

use super::CatalogIssue;
use crate::catalog::CodeNumber;

impl Display for CatalogIssue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName { value } => write!(formatter, "invalid catalog name {value:?}"),
            Self::InvalidPrefix { value } => write!(formatter, "invalid catalog prefix {value:?}"),
            Self::InvalidTypeBase { value } => write!(formatter, "invalid type base {value:?}"),
            Self::TypeNamespaceTooLong { maximum, actual } => write!(
                formatter,
                "catalog type namespace requires {actual} bytes for its largest code; maximum is {maximum}"
            ),
            Self::InvalidMetadata {
                number,
                field,
                reason,
            } => invalid_metadata(formatter, *number, field, reason),
            Self::UnsupportedEvidenceSchema {
                number,
                path,
                reason,
            } => unsupported_schema(formatter, *number, "evidence", path, reason),
            Self::UnsupportedImpactSchema {
                number,
                path,
                reason,
            } => unsupported_schema(formatter, *number, "impact", path, reason),
            Self::DuplicateNumber { number } => duplicate_number(formatter, *number),
            Self::InvalidHttpStatus { number, status } => {
                invalid_http_status(formatter, *number, *status)
            }
            Self::MissingMandatoryHeader {
                number,
                status,
                header,
            } => missing_mandatory_header(formatter, *number, *status, header),
            Self::InvalidTypeUri { number, value } => invalid_type(formatter, *number, value),
            Self::InvalidProblemSetId { value } => {
                write!(formatter, "invalid problem-set operation ID {value:?}")
            }
            Self::DuplicateProblemSetId { id } => duplicate_problem_set(formatter, id),
            Self::DuplicateProblemSetMember {
                problem_set,
                number,
            } => duplicate_problem_set_member(formatter, problem_set, *number),
            Self::UnregisteredProblemSetMember {
                problem_set,
                number,
            } => unregistered_problem_set_member(formatter, problem_set, *number),
            Self::TypeUriTooLong {
                number,
                maximum,
                actual,
            } => type_too_long(formatter, *number, *maximum, *actual),
            Self::InvalidGeneratedArtifact { reason } => {
                write!(formatter, "generated catalog artifact is invalid: {reason}")
            }
        }
    }
}

fn invalid_type(formatter: &mut Formatter<'_>, number: CodeNumber, value: &str) -> fmt::Result {
    write!(
        formatter,
        "diagnostic {number} derives invalid type URI {value:?}"
    )
}

fn duplicate_problem_set(formatter: &mut Formatter<'_>, id: &str) -> fmt::Result {
    write!(
        formatter,
        "problem-set operation ID {id:?} is declared twice"
    )
}

fn duplicate_problem_set_member(
    formatter: &mut Formatter<'_>,
    problem_set: &str,
    number: CodeNumber,
) -> fmt::Result {
    write!(
        formatter,
        "problem set {problem_set:?} includes diagnostic {number} twice"
    )
}

fn unregistered_problem_set_member(
    formatter: &mut Formatter<'_>,
    problem_set: &str,
    number: CodeNumber,
) -> fmt::Result {
    write!(
        formatter,
        "problem set {problem_set:?} includes unregistered HTTP diagnostic {number}"
    )
}

fn type_too_long(
    formatter: &mut Formatter<'_>,
    number: CodeNumber,
    maximum: usize,
    actual: usize,
) -> fmt::Result {
    write!(
        formatter,
        "diagnostic {number} type URI is {actual} bytes; maximum is {maximum}"
    )
}

fn invalid_metadata(
    formatter: &mut Formatter<'_>,
    number: CodeNumber,
    field: &str,
    reason: &str,
) -> fmt::Result {
    write!(
        formatter,
        "diagnostic {number} has invalid {field}: {reason}"
    )
}

fn invalid_http_status(
    formatter: &mut Formatter<'_>,
    number: CodeNumber,
    status: u16,
) -> fmt::Result {
    write!(
        formatter,
        "diagnostic {number} has invalid HTTP status {status}"
    )
}

fn missing_mandatory_header(
    formatter: &mut Formatter<'_>,
    number: CodeNumber,
    status: u16,
    header: &str,
) -> fmt::Result {
    write!(
        formatter,
        "diagnostic {number} status {status} requires header {header}"
    )
}

fn duplicate_number(formatter: &mut Formatter<'_>, number: CodeNumber) -> fmt::Result {
    write!(
        formatter,
        "diagnostic number {number} is declared more than once"
    )
}

fn unsupported_schema(
    formatter: &mut Formatter<'_>,
    number: CodeNumber,
    surface: &str,
    path: &str,
    reason: &str,
) -> fmt::Result {
    write!(
        formatter,
        "diagnostic {number} has unsupported {surface} schema at {path}: {reason}"
    )
}
