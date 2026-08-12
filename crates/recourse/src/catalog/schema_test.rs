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

#[test]
fn malformed_keyword_values_and_patterns_are_rejected() {
    for mut invalid in [
        serde_json::json!({"type": "object", "properties": {"x": {"type": "string", "maxLength": "many"}}}),
        serde_json::json!({"type": "object", "properties": {"x": {"type": "array", "uniqueItems": "yes"}}}),
        serde_json::json!({"type": "object", "properties": {"x": {"type": "number", "multipleOf": 0}}}),
        serde_json::json!({"type": "object", "properties": {"x": {"type": "string", "pattern": "["}}}),
        serde_json::json!({"type": "object", "properties": {"x": {"type": ["string", "string"]}}}),
    ] {
        assert!(
            schema::validate_artifact(&mut invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn supported_formats_are_runtime_assertions_and_unknown_formats_fail() {
    let mut uuid = serde_json::json!({
        "type": "object",
        "properties": {"id": {"type": "string", "format": "uuid"}},
        "required": ["id"],
        "additionalProperties": false
    });
    assert!(schema::validate_artifact(&mut uuid).is_ok());
    let Some(validator) = schema::build_validator(&uuid).ok() else {
        panic!("supported UUID schema must compile");
    };
    assert!(!validator.is_valid(&serde_json::json!({"id": "not-a-uuid"})));

    let mut numeric = serde_json::json!({
        "type": "object",
        "properties": {"count": {"type": "integer", "format": "uint32", "minimum": 0}}
    });
    assert!(schema::validate_artifact(&mut numeric).is_ok());

    let mut misplaced = serde_json::json!({
        "type": "object",
        "properties": {"id": {"type": "string", "format": "uint32"}}
    });
    assert!(schema::validate_artifact(&mut misplaced).is_err());

    let mut unknown = serde_json::json!({
        "type": "object",
        "properties": {"id": {"type": "string", "format": "future-id"}}
    });
    let error = schema::validate_artifact(&mut unknown).err();
    assert!(error.is_some_and(|violation| violation.reason.contains("unsupported format")));
}

#[derive(Debug, Serialize, JsonSchema)]
struct FixedWidthEvidence {
    signed: i32,
    unsigned: u64,
    nullable: Option<u16>,
}

impl PublicEvidence for FixedWidthEvidence {}

#[test]
fn fixed_width_integer_schemas_receive_complete_effective_bounds() {
    let schema = schema::normalize::<FixedWidthEvidence>()
        .unwrap_or_else(|error| panic!("fixed-width schema must normalize: {}", error.reason));

    assert_eq!(
        schema.pointer("/properties/signed/minimum"),
        Some(&serde_json::json!(i32::MIN))
    );
    assert_eq!(
        schema.pointer("/properties/signed/maximum"),
        Some(&serde_json::json!(i32::MAX))
    );
    assert_eq!(
        schema.pointer("/properties/unsigned/maximum"),
        Some(&serde_json::json!(u64::MAX))
    );
    assert_eq!(
        schema.pointer("/properties/nullable/maximum"),
        Some(&serde_json::json!(u16::MAX))
    );
}

#[test]
fn numeric_formats_require_the_exact_json_type_with_optional_null() {
    for mut invalid in [
        serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": "number", "format": "uint8"}}
        }),
        serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": "integer", "format": "float"}}
        }),
        serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": ["integer", "null", "string"], "format": "int32"}}
        }),
    ] {
        assert!(
            schema::validate_artifact(&mut invalid).is_err(),
            "{invalid}"
        );
    }

    let mut nullable = serde_json::json!({
        "type": "object",
        "properties": {"value": {"type": ["integer", "null"], "format": "uint8"}}
    });
    assert!(schema::validate_artifact(&mut nullable).is_ok());
}

#[derive(Debug, Serialize, JsonSchema)]
struct UnsupportedWideEvidence {
    value: u128,
}

impl PublicEvidence for UnsupportedWideEvidence {}

#[test]
fn one_hundred_twenty_eight_bit_and_platform_numeric_formats_are_rejected() {
    let value = UnsupportedWideEvidence { value: u128::MAX };
    assert_eq!(value.value, u128::MAX);
    let violation = schema::normalize::<UnsupportedWideEvidence>().err();
    assert!(violation.is_some_and(|error| error.reason.contains("uint128")));

    for format in ["int", "uint", "int128", "uint128"] {
        let mut invalid = serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": "integer", "format": format}}
        });
        assert!(schema::validate_artifact(&mut invalid).is_err(), "{format}");
    }
}
