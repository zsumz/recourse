//! Conservative rejection of schemas with impossible default-wire instances.

mod depth;
mod satisfiability;

use serde_json::{Map, Value};

use super::{SchemaViolation, fail, number};
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
    depth::validate(schema)
}

fn reject_excessive_minimum(
    schema: &Map<String, Value>,
    keyword: &str,
    maximum: usize,
    path: &str,
) -> Result<(), SchemaViolation> {
    let Some(actual) = schema.get(keyword).and_then(Value::as_number) else {
        return Ok(());
    };
    let maximum_number = serde_json::Number::from(maximum as u64);
    if number::compare(actual, &maximum_number, path)? == std::cmp::Ordering::Greater {
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

fn exact_type(value: Option<&Value>, expected: &str) -> bool {
    matches!(value, Some(Value::String(value)) if value == expected)
}
