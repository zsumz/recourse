//! Governed Problem-set compatibility and lifecycle tests.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{
    AcceptanceMode, Catalog, CatalogArtifact, CatalogLock, CatalogSpec, CodeNumber,
    CompatibilitySeverity, ProblemSet,
};

enum DispatchCatalog {}

impl CatalogSpec for DispatchCatalog {
    const NAME: &'static str = "dispatch";
    const PREFIX: &'static str = "DSP";
    const TYPE_BASE: &'static str = "https://dispatch.invalid/problems/";
}

enum FirstProblem {}
enum SecondProblem {}

macro_rules! problem {
    ($marker:ty, $number:literal, $title:literal) => {
        impl DiagnosticType for $marker {
            type Catalog = DispatchCatalog;
            type Evidence = NoEvidence;

            const NUMBER: CodeNumber = CodeNumber::new($number);
            const TITLE: &'static str = $title;
            const DETAIL: &'static str = "Governed Problem-set fixture.";
            const SUGGESTIONS: &'static [&'static str] = &[];
            const DOCS: &'static str = "Problem-set compatibility fixture.";
        }

        impl HttpProblemType for $marker {
            type Policy = Fixed<400>;
        }
    };
}

problem!(FirstProblem, 1, "First problem");
problem!(SecondProblem, 2, "Second problem");

fn artifact(sets: &[(&str, bool, bool)]) -> CatalogArtifact {
    let mut builder = Catalog::<DispatchCatalog>::builder()
        .problem::<FirstProblem>()
        .problem::<SecondProblem>();
    for (id, first, second) in sets {
        let mut set = ProblemSet::builder(*id);
        if *first {
            set = set.include::<FirstProblem>();
        }
        if *second {
            set = set.include::<SecondProblem>();
        }
        builder = builder.problem_set(set.build());
    }
    builder
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact()
}

#[test]
fn operation_addition_removal_and_rename_are_classified() {
    let previous = artifact(&[("getJob", true, false)]);
    let added = artifact(&[("createJob", true, false), ("getJob", true, false)]);
    let addition = CatalogLock::from_artifact(&previous).check(&added);
    assert!(addition.is_compatible());
    assert!(addition.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-018" && change.severity() == CompatibilitySeverity::Compatible
    }));

    let removal = CatalogLock::from_artifact(&previous).check(&artifact(&[]));
    assert!(removal.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-019" && change.severity() == CompatibilitySeverity::Breaking
    }));

    let rename =
        CatalogLock::from_artifact(&previous).check(&artifact(&[("fetchJob", true, false)]));
    let ids = rename
        .changes()
        .iter()
        .map(super::CompatibilityChange::id)
        .collect::<Vec<_>>();
    assert!(ids.contains(&"REC-COMPAT-018"));
    assert!(ids.contains(&"REC-COMPAT-019"));
}

#[test]
fn member_addition_breaks_while_member_removal_is_compatible() {
    let narrow = artifact(&[("getJob", true, false)]);
    let broad = artifact(&[("getJob", true, true)]);

    let addition = CatalogLock::from_artifact(&narrow).check(&broad);
    assert!(addition.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-020" && change.severity() == CompatibilitySeverity::Breaking
    }));

    let removal = CatalogLock::from_artifact(&broad).check(&narrow);
    assert!(removal.is_compatible());
    assert!(removal.changes().iter().any(|change| {
        change.id() == "REC-COMPAT-021" && change.severity() == CompatibilitySeverity::Compatible
    }));
}

#[test]
fn acceptance_and_retirement_update_the_governed_problem_sets() {
    let narrow = artifact(&[("getJob", true, false)]);
    let broad = artifact(&[("getJob", true, true)]);
    let mut lock = CatalogLock::from_artifact(&narrow);
    assert!(
        lock.accept(&broad, AcceptanceMode::AcknowledgeBreaking)
            .is_ok()
    );
    assert_eq!(lock.problem_sets(), broad.problem_sets());

    let second = "DSP-2"
        .parse()
        .unwrap_or_else(|error| panic!("fixture code must parse: {error}"));
    assert!(lock.retire(&second, "No longer emitted", None).is_ok());
    assert_eq!(
        lock.problem_sets().get("getJob").map(Vec::as_slice),
        Some(
            &["DSP-1"
                .parse()
                .unwrap_or_else(|error| panic!("fixture code must parse: {error}"))][..]
        )
    );
}
