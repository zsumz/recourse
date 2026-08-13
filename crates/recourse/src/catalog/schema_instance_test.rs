//! Default-wire feasibility tests for generated and parsed evidence schemas.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::Serialize;
use serde_json::Map;

use crate::{diagnostic::PublicEvidence, wire::WireLimits};

use super::schema;

#[test]
fn impossible_wire_lower_bounds_and_fixed_values_are_rejected() {
    let required = (0..=WireLimits::DEFAULT_MAX_OBJECT_PROPERTIES)
        .map(|index| format!("field{index}"))
        .collect::<Vec<_>>();
    let mut excessive_required = serde_json::json!({
        "type": "object",
        "required": required,
        "additionalProperties": true
    });
    assert!(schema::validate_artifact(&mut excessive_required).is_err());

    for mut invalid in [
        serde_json::json!({
            "type": "object",
            "properties": {"items": {
                "type": "array",
                "minItems": WireLimits::DEFAULT_MAX_ARRAY_ITEMS + 1
            }}
        }),
        serde_json::json!({
            "type": "object",
            "properties": {"text": {
                "type": "string",
                "minLength": WireLimits::DEFAULT_MAX_STRING_BYTES + 1
            }}
        }),
        serde_json::json!({
            "type": "object",
            "properties": {"fixed": {
                "type": "array",
                "const": vec![0; WireLimits::DEFAULT_MAX_ARRAY_ITEMS + 1]
            }}
        }),
        serde_json::json!({
            "type": "object",
            "properties": {"choice": {
                "type": "string",
                "enum": ["x".repeat(WireLimits::DEFAULT_MAX_STRING_BYTES + 1)]
            }}
        }),
    ] {
        assert!(
            schema::validate_artifact(&mut invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn required_property_names_must_fit_the_wire_string_budget() {
    let name = "x".repeat(WireLimits::DEFAULT_MAX_STRING_BYTES + 1);
    let mut schema_value = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": [name.clone()],
        "additionalProperties": false
    });
    schema_value["properties"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("properties fixture must be an object"))
        .insert(name, serde_json::json!({"type": "string"}));

    assert!(schema::validate_artifact(&mut schema_value).is_err());
}

#[test]
fn provably_mandatory_nesting_must_fit_inside_the_envelope() {
    let mut definitions = Map::new();
    let count = WireLimits::DEFAULT_MAX_NESTING_DEPTH - 1;
    for index in 0..count {
        let child = if index + 1 == count {
            serde_json::json!({"type": "object"})
        } else {
            serde_json::json!({
                "type": "object",
                "properties": {"next": {"$ref": format!("#/$defs/node{}", index + 1)}},
                "required": ["next"]
            })
        };
        definitions.insert(format!("node{index}"), child);
    }
    let mut schema_value = serde_json::json!({
        "type": "object",
        "properties": {"next": {"$ref": "#/$defs/node0"}},
        "required": ["next"],
        "$defs": definitions
    });

    let violation = schema::validate_artifact(&mut schema_value).err();
    assert!(violation.is_some_and(|error| error.reason.contains("mandatory evidence nesting")));
}

#[test]
fn locally_contradictory_constraints_are_rejected() {
    for mut invalid in [
        property(&serde_json::json!({
            "type": "integer", "format": "int8", "minimum": 200
        })),
        property(&serde_json::json!({
            "type": "integer", "format": "uint8", "enum": [300]
        })),
        property(&serde_json::json!({
            "type": "integer", "format": "uint8", "const": 300
        })),
        property(&serde_json::json!({
            "type": "number", "exclusiveMinimum": 10, "maximum": 10
        })),
        property(&serde_json::json!({
            "type": "string", "minLength": 5, "maxLength": 4
        })),
        property(&serde_json::json!({
            "type": "array", "minItems": 2, "maxItems": 1
        })),
    ] {
        assert!(
            schema::validate_artifact(&mut invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn arbitrary_precision_constraints_cannot_hide_impossible_instances() {
    for constraints in [
        r#"{"type":"array","minItems":18446744073709551616}"#,
        r#"{"type":"string","minLength":18446744073709551616}"#,
        r#"{"type":"integer","minimum":18446744073709551617,"maximum":18446744073709551616}"#,
        r#"{"type":"number","format":"double","minimum":1e400}"#,
        r#"{"type":"number","format":"double","maximum":-1e400}"#,
    ] {
        let parsed = serde_json::from_str(constraints)
            .unwrap_or_else(|error| panic!("exact constraint must parse: {error}"));
        let mut invalid = property(&parsed);

        assert!(
            schema::validate_artifact(&mut invalid).is_err(),
            "accepted impossible exact constraint: {constraints}"
        );
    }
}

#[test]
fn arbitrary_precision_comparison_preserves_valid_widening_bounds() {
    let mut schema_value = property(&exact(
        r#"{"type":"number","format":"double","minimum":-1e400,"maximum":1e400}"#,
    ));

    assert!(schema::validate_artifact(&mut schema_value).is_ok());
    assert_eq!(
        schema_value.pointer("/properties/value/minimum"),
        Some(&serde_json::json!(-f64::MAX))
    );
    assert_eq!(
        schema_value.pointer("/properties/value/maximum"),
        Some(&serde_json::json!(f64::MAX))
    );

    for constraints in [
        r#"{"type":"number","minimum":1e400,"maximum":9e399}"#,
        r#"{"type":"number","minimum":-9e399,"maximum":-1e400}"#,
        r#"{"type":"number","minimum":1.200,"exclusiveMaximum":1.2}"#,
    ] {
        let mut invalid = property(&exact(constraints));
        assert!(schema::validate_artifact(&mut invalid).is_err());
    }
}

#[test]
fn a_partially_valid_enum_remains_satisfiable() {
    let mut value = property(&serde_json::json!({
        "type": "integer", "format": "uint8", "enum": [10, 300]
    }));

    assert!(schema::validate_artifact(&mut value).is_ok());

    let mut mixed_precision = property(&serde_json::json!({
        "type": "number",
        "exclusiveMinimum": 9_007_199_254_740_992.0,
        "maximum": 9_007_199_254_740_993_u64
    }));
    assert!(schema::validate_artifact(&mut mixed_precision).is_ok());
}

fn property(constraints: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {"value": constraints},
        "required": ["value"],
        "additionalProperties": false
    })
}

fn exact(encoded: &str) -> serde_json::Value {
    serde_json::from_str(encoded)
        .unwrap_or_else(|error| panic!("exact constraint must parse: {error}"))
}

#[derive(Debug, Serialize)]
struct ExactNumberEvidence;

impl JsonSchema for ExactNumberEvidence {
    fn schema_name() -> Cow<'static, str> {
        "ExactNumberEvidence".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        serde_json::from_str(
            r#"{
                "type":"object",
                "properties":{
                    "items":{"type":"array","minItems":18446744073709551616}
                },
                "required":["items"],
                "additionalProperties":false
            }"#,
        )
        .unwrap_or_else(|error| panic!("exact generated schema must parse: {error}"))
    }
}

impl PublicEvidence for ExactNumberEvidence {}

#[test]
fn generated_schemas_cannot_require_values_beyond_wire_limits() {
    let violation = schema::normalize::<ExactNumberEvidence>().err();
    assert!(violation.is_some_and(|error| error.reason.contains("default wire maximum")));
}
