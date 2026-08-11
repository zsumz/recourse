//! Aggregated catalog definition failures with precise ownership context.

use std::fmt::{self, Display, Formatter};

use super::CodeNumber;

/// One independently actionable catalog definition problem.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogIssue {
    /// Catalog name is not canonical lowercase kebab case.
    InvalidName {
        /// Rejected declaration.
        value: String,
    },
    /// Catalog prefix is not a canonical code prefix.
    InvalidPrefix {
        /// Rejected declaration.
        value: String,
    },
    /// Catalog type base is not an absolute URI ending in `/`.
    InvalidTypeBase {
        /// Rejected declaration.
        value: String,
    },
    /// A required metadata field is empty or otherwise invalid.
    InvalidMetadata {
        /// Diagnostic number with invalid metadata.
        number: CodeNumber,
        /// Stable metadata field name.
        field: &'static str,
        /// Human-readable reason for the definition author.
        reason: String,
    },
    /// Evidence schema is outside the supported deterministic profile.
    UnsupportedEvidenceSchema {
        /// Diagnostic number owning the evidence type.
        number: CodeNumber,
        /// JSON-pointer-like location within the schema.
        path: String,
        /// Human-readable reason for the definition author.
        reason: String,
    },
    /// Operation impact schema is outside the supported deterministic profile.
    UnsupportedImpactSchema {
        /// Diagnostic number owning the impact type.
        number: CodeNumber,
        /// JSON-pointer-like location within the schema.
        path: String,
        /// Human-readable reason for the definition author.
        reason: String,
    },
    /// Two different diagnostic marker types claim one permanent number.
    DuplicateNumber {
        /// Conflicting permanent number.
        number: CodeNumber,
    },
    /// HTTP status is not a valid client- or server-error status.
    InvalidHttpStatus {
        /// Diagnostic number owning the policy.
        number: CodeNumber,
        /// Rejected status value.
        status: u16,
    },
    /// HTTP policy omits a header mandated by its status.
    MissingMandatoryHeader {
        /// Diagnostic number owning the policy.
        number: CodeNumber,
        /// Status whose semantics require the header.
        status: u16,
        /// Missing canonical header name.
        header: &'static str,
    },
    /// A derived type URI is not a valid absolute URI.
    InvalidTypeUri {
        /// Diagnostic number whose URI could not be derived safely.
        number: CodeNumber,
        /// Rejected derived value.
        value: String,
    },
    /// Problem-set operation ID is empty, unsafe, or too long.
    InvalidProblemSetId {
        /// Rejected operation ID.
        value: String,
    },
    /// Two declarations claim the same stable API operation ID.
    DuplicateProblemSetId {
        /// Repeated operation ID.
        id: String,
    },
    /// One problem set includes the same diagnostic more than once.
    DuplicateProblemSetMember {
        /// Owning operation ID.
        problem_set: String,
        /// Repeated diagnostic number.
        number: CodeNumber,
    },
    /// A problem set includes a marker not registered on the HTTP surface.
    UnregisteredProblemSetMember {
        /// Owning operation ID.
        problem_set: String,
        /// Missing HTTP diagnostic number.
        number: CodeNumber,
    },
}

impl Display for CatalogIssue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName { value } => write!(formatter, "invalid catalog name {value:?}"),
            Self::InvalidPrefix { value } => write!(formatter, "invalid catalog prefix {value:?}"),
            Self::InvalidTypeBase { value } => write!(formatter, "invalid type base {value:?}"),
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
            Self::InvalidTypeUri { number, value } => {
                write!(
                    formatter,
                    "diagnostic {number} derives invalid type URI {value:?}"
                )
            }
            Self::InvalidProblemSetId { value } => {
                write!(formatter, "invalid problem-set operation ID {value:?}")
            }
            Self::DuplicateProblemSetId { id } => {
                write!(
                    formatter,
                    "problem-set operation ID {id:?} is declared twice"
                )
            }
            Self::DuplicateProblemSetMember {
                problem_set,
                number,
            } => write!(
                formatter,
                "problem set {problem_set:?} includes diagnostic {number} twice"
            ),
            Self::UnregisteredProblemSetMember {
                problem_set,
                number,
            } => write!(
                formatter,
                "problem set {problem_set:?} includes unregistered HTTP diagnostic {number}"
            ),
        }
    }
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
