//! Focused tests for the reviewed public-evidence boundary.

use schemars::schema_for;

use super::{NoEvidence, PublicEvidence};

#[test]
fn no_evidence_is_an_empty_json_object() {
    assert!(matches!(
        serde_json::to_value(NoEvidence),
        Ok(serde_json::Value::Object(fields)) if fields.is_empty()
    ));
}

#[test]
fn no_evidence_schema_has_an_object_root() {
    let schema = schema_for!(NoEvidence);
    let encoded = serde_json::to_value(schema);

    assert!(matches!(
        encoded,
        Ok(serde_json::Value::Object(fields))
            if fields.get("type") == Some(&serde_json::Value::String("object".to_owned()))
    ));
}

#[test]
fn no_evidence_schema_pins_the_generated_catalog_shape() {
    let encoded = serde_json::to_value(schema_for!(NoEvidence));

    assert_eq!(
        encoded.ok(),
        Some(serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "NoEvidence",
            "description": "Empty public evidence, represented on the wire as `{}`.",
            "type": "object"
        }))
    );
}

#[test]
fn no_evidence_round_trips_and_ignores_unknown_members() {
    let empty = serde_json::from_str::<NoEvidence>("{}");
    let extended = serde_json::from_str::<NoEvidence>(r#"{"added_later":1}"#);
    let scalar = serde_json::from_str::<NoEvidence>("null");

    assert_eq!(empty.ok(), Some(NoEvidence));
    assert_eq!(extended.ok(), Some(NoEvidence));
    assert!(scalar.is_err());
    assert_eq!(
        serde_json::to_string(&NoEvidence).ok().as_deref(),
        Some("{}")
    );
}

#[test]
fn no_evidence_satisfies_the_explicit_marker_contract() {
    fn assert_public<T: PublicEvidence>() {}

    assert_public::<NoEvidence>();
}
