//! Dispatch namespace and explicit diagnostic registration.

use recourse::catalog::{Catalog, CatalogBuildError, CatalogSpec};

use crate::{
    AuthenticationRequired, DispatchFailed, IdempotencyConflict, InternalError, JobNotFound,
    MalformedRequest, QueueUnavailable, ServiceTemporarilyUnavailable, UnsupportedMediaType,
    UnsupportedMethod, ValidationFailed, create_job_problems, get_job_problems,
};

/// Stable namespace marker for every Dispatch diagnostic.
#[derive(Debug)]
pub enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

/// Builds the complete explicitly registered Dispatch catalog.
pub fn catalog() -> Result<Catalog<DispatchCatalog>, CatalogBuildError> {
    Catalog::<DispatchCatalog>::builder()
        .problem::<MalformedRequest>()
        .problem::<ValidationFailed>()
        .problem::<JobNotFound>()
        .problem::<IdempotencyConflict>()
        .problem::<AuthenticationRequired>()
        .problem::<UnsupportedMethod>()
        .problem::<ServiceTemporarilyUnavailable>()
        .problem::<InternalError>()
        .operation::<DispatchFailed>()
        .problem::<QueueUnavailable>()
        .health::<QueueUnavailable>()
        .problem::<UnsupportedMediaType>()
        .problem_set(create_job_problems())
        .problem_set(get_job_problems())
        .build()
}
