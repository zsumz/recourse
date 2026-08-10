//! Focused tests for the conservative evidence-schema profile.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

use crate::{diagnostic::PublicEvidence, validation::ValidationEvidence};

use super::schema;

#[derive(Debug, Serialize, JsonSchema)]
struct OrderedEvidence {
    zeta: String,
    alpha: u64,
    optional: Option<bool>,
}

impl PublicEvidence for OrderedEvidence {}

#[test]
fn normalization_removes_annotations_and_sorts_required_fields() {
    let normalized = schema::normalize::<OrderedEvidence>();
    let Some(Value::Object(object)) = normalized.ok() else {
        return;
    };

    assert!(!object.contains_key("$schema"));
    assert!(!object.contains_key("title"));
    assert_eq!(
        object.get("required"),
        Some(&serde_json::json!(["alpha", "zeta"]))
    );
}

#[derive(Debug, Serialize, JsonSchema)]
struct ScalarEvidence(String);

impl PublicEvidence for ScalarEvidence {}

#[test]
fn root_scalars_are_rejected() {
    let violation = schema::normalize::<ScalarEvidence>().err();

    assert!(violation.is_some_and(|value| value.reason.contains("object root")));
}

#[derive(Debug, Serialize, JsonSchema)]
struct MapEvidence(BTreeMap<String, String>);

impl PublicEvidence for MapEvidence {}

#[test]
fn unbounded_root_maps_are_rejected() {
    let violation = schema::normalize::<MapEvidence>().err();

    assert!(violation.is_some_and(|value| value.reason.contains("root arbitrary maps")));
}

#[test]
fn built_in_validation_evidence_fits_the_profile() {
    let normalized = schema::normalize::<ValidationEvidence>();

    assert!(normalized.is_ok());
    assert_eq!(
        normalized
            .ok()
            .and_then(|value| value.pointer("/properties/violations/maxItems").cloned()),
        Some(serde_json::json!(100))
    );
}
