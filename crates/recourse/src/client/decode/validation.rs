//! Complete JSON-tree traversal enforcing local and semantic collection limits.

use serde_json::Value;

use super::{DecodeError, DecodeLimit, DecodeLimits};

pub(super) fn validate_shape(value: &Value, limits: DecodeLimits) -> Result<(), DecodeError> {
    validate_value(value, limits, 0)?;
    validate_named_array(
        value,
        "suggestions",
        DecodeLimit::Suggestions,
        limits.max_suggestions(),
    )?;
    if let Some(evidence) = value.get("evidence") {
        validate_named_array(
            evidence,
            "violations",
            DecodeLimit::Violations,
            limits.max_violations(),
        )?;
    }
    Ok(())
}

fn validate_value(value: &Value, limits: DecodeLimits, depth: usize) -> Result<(), DecodeError> {
    match value {
        Value::Object(object) => {
            enforce(
                DecodeLimit::NestingDepth,
                limits.max_nesting_depth(),
                depth + 1,
            )?;
            enforce(
                DecodeLimit::ObjectProperties,
                limits.max_object_properties(),
                object.len(),
            )?;
            for (key, child) in object {
                enforce(
                    DecodeLimit::StringBytes,
                    limits.max_string_bytes(),
                    key.len(),
                )?;
                validate_value(child, limits, depth + 1)?;
            }
        }
        Value::Array(array) => {
            enforce(
                DecodeLimit::NestingDepth,
                limits.max_nesting_depth(),
                depth + 1,
            )?;
            enforce(
                DecodeLimit::ArrayItems,
                limits.max_array_items(),
                array.len(),
            )?;
            for child in array {
                validate_value(child, limits, depth + 1)?;
            }
        }
        Value::String(value) => {
            enforce(
                DecodeLimit::StringBytes,
                limits.max_string_bytes(),
                value.len(),
            )?;
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn validate_named_array(
    parent: &Value,
    name: &str,
    limit: DecodeLimit,
    maximum: usize,
) -> Result<(), DecodeError> {
    let Some(Value::Array(values)) = parent.get(name) else {
        return Ok(());
    };
    enforce(limit, maximum, values.len())
}

fn enforce(limit: DecodeLimit, maximum: usize, actual: usize) -> Result<(), DecodeError> {
    if actual > maximum {
        return Err(DecodeError::LimitExceeded {
            limit,
            maximum,
            actual,
        });
    }
    Ok(())
}
