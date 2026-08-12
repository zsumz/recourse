//! Default-wire feasibility tests for accepted evidence schemas.

use serde_json::Map;

use crate::wire::WireLimits;

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
