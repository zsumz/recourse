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
fn current_locks_write_schema_version_two_with_governed_problem_sets() {
    let value = lock_value();

    assert_eq!(value["schema_version"], serde_json::json!(2));
    assert_eq!(
        value["problem_sets"]["operation"],
        serde_json::json!(["TST-1"])
    );
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    assert!(CatalogLock::from_slice(&body).is_ok());
}

#[test]
fn schema_version_one_locks_migrate_to_version_two_with_empty_problem_sets() {
    let mut value = lock_value();
    value["schema_version"] = serde_json::json!(1);
    value
        .as_object_mut()
        .unwrap_or_else(|| panic!("fixture lock must be an object"))
        .remove("problem_sets");
    let body = serde_json::to_vec(&value)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    let parsed = CatalogLock::from_slice(&body)
        .unwrap_or_else(|error| panic!("legacy lock must parse: {error}"));

    assert_eq!(parsed.schema_version(), 2);
    assert!(parsed.problem_sets().is_empty());
    let written = serde_json::to_value(parsed)
        .unwrap_or_else(|error| panic!("migrated lock must encode: {error}"));
    assert_eq!(written["schema_version"], serde_json::json!(2));
    assert_eq!(written["problem_sets"], serde_json::json!({}));
}

#[test]
fn lock_schema_versions_name_their_exact_top_level_shape() {
    let mut version_one_with_sets = lock_value();
    version_one_with_sets["schema_version"] = serde_json::json!(1);
    let body = serde_json::to_vec(&version_one_with_sets)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    assert!(CatalogLock::from_slice(&body).is_err());

    version_one_with_sets["problem_sets"] = serde_json::Value::Null;
    let body = serde_json::to_vec(&version_one_with_sets)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    assert!(CatalogLock::from_slice(&body).is_err());

    let mut version_two_without_sets = lock_value();
    version_two_without_sets
        .as_object_mut()
        .unwrap_or_else(|| panic!("fixture lock must be an object"))
        .remove("problem_sets");
    let body = serde_json::to_vec(&version_two_without_sets)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    assert!(CatalogLock::from_slice(&body).is_err());

    version_two_without_sets["problem_sets"] = serde_json::Value::Null;
    let body = serde_json::to_vec(&version_two_without_sets)
        .unwrap_or_else(|error| panic!("fixture lock must encode: {error}"));
    assert!(CatalogLock::from_slice(&body).is_err());
}
