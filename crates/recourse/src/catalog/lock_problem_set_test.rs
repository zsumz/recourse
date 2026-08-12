//! Lock-parser boundaries for governed Problem sets.

use crate::{
    diagnostic::{DiagnosticType, NoEvidence},
    http::{Fixed, HttpProblemType},
};

use super::{Catalog, CatalogLock, CatalogSpec, CodeNumber, ProblemSet};

enum TestCatalog {}

impl CatalogSpec for TestCatalog {
    const NAME: &'static str = "test";
    const PREFIX: &'static str = "TST";
    const TYPE_BASE: &'static str = "https://test.invalid/problems/";
}

enum KnownProblem {}

impl DiagnosticType for KnownProblem {
    type Catalog = TestCatalog;
    type Evidence = NoEvidence;

    const NUMBER: CodeNumber = CodeNumber::new(1);
    const TITLE: &'static str = "Known problem";
    const DETAIL: &'static str = "Known Problem-set member.";
    const SUGGESTIONS: &'static [&'static str] = &[];
    const DOCS: &'static str = "Lock Problem-set fixture.";
}

impl HttpProblemType for KnownProblem {
    type Policy = Fixed<400>;
}

fn lock_value() -> serde_json::Value {
    let set = ProblemSet::builder("operation")
        .include::<KnownProblem>()
        .build();
    let artifact = Catalog::<TestCatalog>::builder()
        .problem::<KnownProblem>()
        .problem_set(set)
        .build()
        .unwrap_or_else(|error| panic!("fixture catalog must build: {error}"))
        .artifact();
    serde_json::to_value(CatalogLock::from_artifact(&artifact))
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"))
}

#[test]
fn parser_requires_every_problem_set_member_to_be_active() {
    let mut value = lock_value();
    value["problem_sets"]["operation"] = serde_json::json!(["TST-2"]);
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));

    assert!(CatalogLock::from_slice(&body).is_err());
}

#[test]
fn locks_from_before_problem_set_governance_default_to_an_empty_map() {
    let mut value = lock_value();
    value
        .as_object_mut()
        .unwrap_or_else(|| panic!("fixture lock must be an object"))
        .remove("problem_sets");
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    let parsed = CatalogLock::from_slice(&body)
        .unwrap_or_else(|error| panic!("legacy lock must parse: {error}"));

    assert!(parsed.problem_sets().is_empty());
}
