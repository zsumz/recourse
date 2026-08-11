//! The published compatibility profile stays complete and behavior-backed.

use std::collections::BTreeSet;

use super::CompatibilityReport;

const IMPLEMENTATION: &str = concat!(
    include_str!("lock/compatibility/change/input/lifecycle.rs"),
    include_str!("lock/compatibility/change/input/diagnostic.rs"),
    include_str!("lock/compatibility/change/input/schema.rs"),
);
const BEHAVIOR: &str = concat!(
    include_str!("compatibility_identity_test.rs"),
    include_str!("compatibility_schema_test.rs"),
    include_str!("compatibility_test.rs"),
);

#[test]
fn profile_names_every_implemented_and_exercised_rule() {
    let profile: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/compatibility/profile.json"
    ))
    .unwrap_or_else(|error| panic!("compatibility profile must be JSON: {error}"));
    assert_eq!(profile["profile"], "recourse-0.0.1");
    let profile_ids = profile["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|rule| rule["id"].as_str())
        .collect::<BTreeSet<_>>();
    let implemented_ids = stable_ids(IMPLEMENTATION);
    let behavior_ids = stable_ids(BEHAVIOR);

    assert_eq!(profile_ids, implemented_ids);
    assert!(profile_ids.is_subset(&behavior_ids));
    assert_eq!(profile_ids.len(), 17);
}

fn stable_ids(source: &str) -> BTreeSet<&str> {
    source
        .match_indices("REC-COMPAT-")
        .filter_map(|(start, _)| source.get(start..start + 14))
        .filter(|value| value[11..].bytes().all(|byte| byte.is_ascii_digit()))
        .collect()
}

pub(super) fn assert_report_fixture(report: &CompatibilityReport, fixture: &str) {
    let actual = serde_json::to_value(report)
        .unwrap_or_else(|error| panic!("compatibility report must encode: {error}"));
    let expected: serde_json::Value = serde_json::from_str(fixture)
        .unwrap_or_else(|error| panic!("compatibility fixture must be JSON: {error}"));
    assert_eq!(actual, expected);
}
