//! Conservative rejection of schemas with impossible default-wire instances.

mod satisfiability;

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::{SchemaViolation, fail};
use crate::wire::{BoundedJsonError, WireLimits, to_bounded_vec, validate_embedded};

pub(super) fn validate_local(
    schema: &Map<String, Value>,
    path: &str,
) -> Result<(), SchemaViolation> {
    let limits = WireLimits::default();
    if exact_type(schema.get("type"), "array") {
        reject_excessive_minimum(schema, "minItems", limits.max_array_items(), path)?;
    }
    if exact_type(schema.get("type"), "string") {
        reject_excessive_minimum(schema, "minLength", limits.max_string_bytes(), path)?;
    }
    if exact_type(schema.get("type"), "object") {
        validate_required(schema.get("required"), limits, path)?;
    }
    if let Some(value) = schema.get("const") {
        validate_fixed_value(value, limits, &format!("{path}/const"))?;
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        for (index, value) in values.iter().enumerate() {
            validate_fixed_value(value, limits, &format!("{path}/enum/{index}"))?;
        }
    }
    satisfiability::validate(schema, path)
}

pub(super) fn validate_depth(schema: &Value) -> Result<(), SchemaViolation> {
    let depth = minimum_container_depth(schema, schema, &mut BTreeSet::new());
    let actual = depth.saturating_add(1);
    let maximum = WireLimits::DEFAULT_MAX_NESTING_DEPTH;
    if actual > maximum {
        fail(
            "$",
            &format!(
                "mandatory evidence nesting requires depth {actual}; default wire maximum is {maximum}"
            ),
        )
    } else {
        Ok(())
    }
}

fn reject_excessive_minimum(
    schema: &Map<String, Value>,
    keyword: &str,
    maximum: usize,
    path: &str,
) -> Result<(), SchemaViolation> {
    let Some(actual) = schema.get(keyword).and_then(Value::as_u64) else {
        return Ok(());
    };
    if actual > maximum as u64 {
        fail(
            path,
            &format!("{keyword} {actual} exceeds default wire maximum {maximum}"),
        )
    } else {
        Ok(())
    }
}

fn validate_required(
    required: Option<&Value>,
    limits: WireLimits,
    path: &str,
) -> Result<(), SchemaViolation> {
    let Some(required) = required.and_then(Value::as_array) else {
        return Ok(());
    };
    if required.len() > limits.max_object_properties() {
        return fail(
            path,
            &format!(
                "required property count {} exceeds default wire maximum {}",
                required.len(),
                limits.max_object_properties()
            ),
        );
    }
    if let Some(name) = required
        .iter()
        .filter_map(Value::as_str)
        .find(|name| name.len() > limits.max_string_bytes())
    {
        return fail(
            path,
            &format!(
                "required property name is {} bytes; default wire maximum is {}",
                name.len(),
                limits.max_string_bytes()
            ),
        );
    }
    Ok(())
}

fn validate_fixed_value(
    value: &Value,
    limits: WireLimits,
    path: &str,
) -> Result<(), SchemaViolation> {
    validate_embedded(value, limits).map_err(|error| SchemaViolation {
        path: path.to_owned(),
        reason: format!("fixed value exceeds default wire limits: {error}"),
    })?;
    match to_bounded_vec(value, limits) {
        Ok(_) => Ok(()),
        Err(BoundedJsonError::Limit(error)) => Err(SchemaViolation {
            path: path.to_owned(),
            reason: format!("fixed value exceeds default wire limits: {error}"),
        }),
        Err(BoundedJsonError::Serialize(error)) => Err(SchemaViolation {
            path: path.to_owned(),
            reason: format!("fixed value cannot be encoded: {error}"),
        }),
    }
}

fn minimum_container_depth(
    schema: &Value,
    root: &Value,
    references: &mut BTreeSet<String>,
) -> usize {
    let Some(object) = schema.as_object() else {
        return 0;
    };
    if let Some(value) = object.get("const") {
        return value_depth(value);
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return values.iter().map(value_depth).min().unwrap_or(0);
    }
    let mut depth = type_depth(object, root, references);
    for keyword in ["anyOf", "oneOf"] {
        let choice = object
            .get(keyword)
            .and_then(Value::as_array)
            .and_then(|values| {
                values
                    .iter()
                    .map(|value| minimum_container_depth(value, root, references))
                    .min()
            })
            .unwrap_or(0);
        depth = depth.max(choice);
    }
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        if !references.insert(reference.to_owned()) {
            return usize::MAX;
        }
        let referenced = root.pointer(reference.trim_start_matches('#'));
        let reference_depth =
            referenced.map_or(0, |value| minimum_container_depth(value, root, references));
        references.remove(reference);
        depth = depth.max(reference_depth);
    }
    depth
}

fn type_depth(
    schema: &Map<String, Value>,
    root: &Value,
    references: &mut BTreeSet<String>,
) -> usize {
    if exact_type(schema.get("type"), "object") {
        let child = schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|name| schema.get("properties")?.get(name))
            .map(|value| minimum_container_depth(value, root, references))
            .max()
            .unwrap_or(0);
        1usize.saturating_add(child)
    } else if exact_type(schema.get("type"), "array") {
        let child = if schema.get("minItems").and_then(Value::as_u64) > Some(0) {
            schema
                .get("items")
                .map_or(0, |value| minimum_container_depth(value, root, references))
        } else {
            0
        };
        1usize.saturating_add(child)
    } else {
        0
    }
}

fn exact_type(value: Option<&Value>, expected: &str) -> bool {
    matches!(value, Some(Value::String(value)) if value == expected)
}

fn value_depth(value: &Value) -> usize {
    match value {
        Value::Object(object) => {
            1usize.saturating_add(object.values().map(value_depth).max().unwrap_or(0))
        }
        Value::Array(values) => {
            1usize.saturating_add(values.iter().map(value_depth).max().unwrap_or(0))
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => 0,
    }
}
