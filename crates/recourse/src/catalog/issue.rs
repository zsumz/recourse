//! Aggregated catalog definition failures with precise ownership context.

mod display;

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
    /// A derived type URI exceeds the default diagnostic wire profile.
    TypeUriTooLong {
        /// Diagnostic number whose URI is too large.
        number: CodeNumber,
        /// Maximum accepted UTF-8 byte length.
        maximum: usize,
        /// Actual UTF-8 byte length.
        actual: usize,
    },
    /// Recourse could not encode and reparse its generated artifact exactly.
    InvalidGeneratedArtifact {
        /// Actionable closure failure.
        reason: String,
    },
    /// The permanent namespace cannot represent every positive `u32` code.
    TypeNamespaceTooLong {
        /// Maximum accepted UTF-8 byte length.
        maximum: usize,
        /// Length of the type URI derived for `u32::MAX`.
        actual: usize,
    },
}
