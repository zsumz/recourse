//! Permanent catalog identity and namespace declarations.

mod artifact;
mod builder;
mod code;
mod issue;
mod lock;
mod problem_set;
mod schema;
mod spec;

pub use artifact::{ArtifactParseError, ArtifactWriteError, CatalogArtifact, CatalogDiagnostic};
pub use builder::{Catalog, CatalogBuilder};
pub use code::{Code, CodeNumber, CodeNumberError, CodeParseError};
pub use issue::{CatalogBuildError, CatalogIssue};
pub use lock::{
    AcceptanceError, AcceptanceMode, CatalogLock, CompatibilityChange, CompatibilityReport,
    CompatibilitySeverity, LockEntry, LockParseError, LockState, LockWriteError, Reservation,
    ReservationError, RetirementError,
};
pub(crate) use problem_set::valid_problem_set_id;
pub use problem_set::{MAX_PROBLEM_SET_ID_BYTES, ProblemSet, ProblemSetBuilder};
pub use spec::CatalogSpec;

#[cfg(test)]
mod artifact_parse_test;
#[cfg(test)]
mod artifact_test;
#[cfg(test)]
mod builder_test;
#[cfg(test)]
mod code_test;
#[cfg(test)]
mod compatibility_identity_test;
#[cfg(test)]
mod compatibility_profile_test;
#[cfg(test)]
mod compatibility_schema_test;
#[cfg(test)]
mod compatibility_test;
#[cfg(test)]
mod lock_test;
#[cfg(test)]
mod problem_set_test;
#[cfg(test)]
mod schema_test;
#[cfg(test)]
mod surface_test;
