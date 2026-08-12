//! Permanent catalog identity and namespace declarations.

mod artifact;
mod build_error;
mod builder;
mod code;
mod issue;
mod lock;
mod metadata;
mod problem_set;
mod schema;
mod spec;
mod type_uri;
mod validator;

pub use artifact::{
    ArtifactParseError, ArtifactWriteError, CatalogArtifact, CatalogDiagnostic,
    MAX_CATALOG_ARTIFACT_BYTES,
};
pub use build_error::CatalogBuildError;
pub use builder::{Catalog, CatalogBuilder};
pub use code::{Code, CodeNumber, CodeNumberError, CodeParseError};
pub use issue::CatalogIssue;
pub use lock::{
    AcceptanceError, AcceptanceMode, CatalogLock, CompatibilityChange, CompatibilityReport,
    CompatibilitySeverity, LockEntry, LockParseError, LockState, LockWriteError,
    MAX_CATALOG_LOCK_BYTES, MAX_CATALOG_LOCK_ENTRIES, MAX_RETIREMENT_REASON_CHARS, Reservation,
    ReservationError, RetirementError,
};
pub use metadata::{
    MAX_DETAIL_CHARS, MAX_DOCUMENTATION_CHARS, MAX_SUGGESTION_CHARS, MAX_SUGGESTIONS,
    MAX_TITLE_CHARS,
};
pub(crate) use problem_set::valid_problem_set_id;
pub use problem_set::{MAX_PROBLEM_SET_ID_BYTES, ProblemSet, ProblemSetBuilder};
pub use schema::{SUPPORTED_SCHEMA_FORMATS, SUPPORTED_SCHEMA_NUMERIC_FORMATS};
pub use spec::CatalogSpec;
pub(crate) use type_uri::{
    maximum_type_uri_bytes, type_namespace_fits_wire, valid_type_base, valid_type_uri,
};
pub(crate) use validator::{DiagnosticValidators, validate as validate_value};

#[cfg(test)]
mod artifact_parse_test;
#[cfg(test)]
mod artifact_test;
#[cfg(test)]
mod builder_limit_test;
#[cfg(test)]
mod builder_test;
#[cfg(test)]
mod code_test;
#[cfg(test)]
mod compatibility_identity_test;
#[cfg(test)]
mod compatibility_problem_set_test;
#[cfg(test)]
mod compatibility_profile_test;
#[cfg(test)]
mod compatibility_schema_test;
#[cfg(test)]
mod compatibility_test;
#[cfg(test)]
mod lock_problem_set_test;
#[cfg(test)]
mod lock_test;
#[cfg(test)]
mod metadata_test;
#[cfg(test)]
mod problem_set_test;
#[cfg(test)]
mod retirement_lock_test;
#[cfg(test)]
mod schema_instance_test;
#[cfg(test)]
mod schema_test;
#[cfg(test)]
mod surface_test;
#[cfg(test)]
mod type_uri_test;
