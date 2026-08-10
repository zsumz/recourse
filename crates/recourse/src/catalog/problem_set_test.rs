//! Problem-set validation and deterministic artifact membership tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogIssue, CatalogSpec, CodeNumber, ProblemSet};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum JobNotFound {}

impl DiagnosticType for JobNotFound {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1003);
    const TITLE: &'static str = "Job not found";
    const DETAIL: &'static str = "No job exists for the supplied identifier.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Check the supplied job identifier.";
}

impl HttpProblemType for JobNotFound {
    type Policy = Fixed<404>;
}

enum Conflict {}

impl DiagnosticType for Conflict {
    type Catalog = DispatchCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1004);
    const TITLE: &'static str = "Conflict";
    const DETAIL: &'static str = "The operation conflicts with existing state.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Change the request before retrying.";
}

impl HttpProblemType for Conflict {
    type Policy = Fixed<409>;
}

fn create_job() -> ProblemSet<DispatchCatalog> {
    ProblemSet::builder("createJob")
        .include::<Conflict>()
        .include::<JobNotFound>()
        .build()
}

#[test]
fn artifact_sorts_operation_ids_and_member_codes() {
    let catalog = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .problem::<Conflict>()
        .problem_set(create_job())
        .build()
        .unwrap_or_else(|error| panic!("problem set must build: {error}"));

    let artifact = catalog.artifact();
    let codes = artifact.problem_sets().get("createJob");
    assert_eq!(
        codes.map(|values| values.iter().map(ToString::to_string).collect::<Vec<_>>()),
        Some(vec!["DSP-1003".to_owned(), "DSP-1004".to_owned()])
    );
}

#[test]
fn invalid_duplicate_and_unregistered_members_fail_together() {
    let repeated = ProblemSet::builder("createJob")
        .include::<JobNotFound>()
        .include::<JobNotFound>()
        .include::<Conflict>()
        .build();
    let duplicate_id = ProblemSet::<DispatchCatalog>::builder("createJob").build();
    let invalid_id = ProblemSet::<DispatchCatalog>::builder("/jobs").build();
    let error = Catalog::<DispatchCatalog>::builder()
        .problem::<JobNotFound>()
        .problem_set(repeated)
        .problem_set(duplicate_id)
        .problem_set(invalid_id)
        .build()
        .err()
        .unwrap_or_else(|| panic!("invalid sets must fail"));

    assert!(error.issues().iter().any(|issue| matches!(
        issue,
        CatalogIssue::DuplicateProblemSetMember { number, .. }
            if *number == CodeNumber::new(1003)
    )));
    assert!(error.issues().iter().any(|issue| matches!(
        issue,
        CatalogIssue::UnregisteredProblemSetMember { number, .. }
            if *number == CodeNumber::new(1004)
    )));
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, CatalogIssue::DuplicateProblemSetId { .. }))
    );
    assert!(
        error
            .issues()
            .iter()
            .any(|issue| matches!(issue, CatalogIssue::InvalidProblemSetId { .. }))
    );
}
