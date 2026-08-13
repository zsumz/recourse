//! Complete JSON-tree validation under the shared wire profile.

use serde_json::Value;

use super::{WireLimit, WireLimitError, WireLimits};

pub(crate) fn validate_value(value: &Value, limits: WireLimits) -> Result<(), WireLimitError> {
    validate_subtree(value, limits, 0)?;
    if let Some(suggestions) = value.get("suggestions") {
        validate_named_array(
            suggestions,
            WireLimit::Suggestions,
            limits.max_suggestions(),
        )?;
    }
    if let Some(evidence) = value.get("evidence") {
        validate_violations(evidence, limits)?;
    }
    Ok(())
}

pub(crate) fn validate_evidence(
    evidence: &Value,
    limits: WireLimits,
) -> Result<(), WireLimitError> {
    validate_embedded(evidence, limits)?;
    validate_violations(evidence, limits)
}

pub(crate) fn validate_embedded(value: &Value, limits: WireLimits) -> Result<(), WireLimitError> {
    validate_subtree(value, limits, 1)
}

pub(crate) fn validate_wire_parts(
    property_count: usize,
    strings: &[&str],
    suggestions: &[String],
    limits: WireLimits,
) -> Result<(), WireLimitError> {
    enforce(WireLimit::NestingDepth, limits.max_nesting_depth(), 1)?;
    enforce(
        WireLimit::ObjectProperties,
        limits.max_object_properties(),
        property_count,
    )?;
    enforce(
        WireLimit::Suggestions,
        limits.max_suggestions(),
        suggestions.len(),
    )?;
    enforce(
        WireLimit::ArrayItems,
        limits.max_array_items(),
        suggestions.len(),
    )?;
    for value in strings
        .iter()
        .copied()
        .chain(suggestions.iter().map(String::as_str))
    {
        enforce(
            WireLimit::StringBytes,
            limits.max_string_bytes(),
            value.len(),
        )?;
    }
    Ok(())
}

fn validate_subtree(value: &Value, limits: WireLimits, depth: usize) -> Result<(), WireLimitError> {
    match value {
        Value::Object(object) => {
            enforce(
                WireLimit::NestingDepth,
                limits.max_nesting_depth(),
                depth + 1,
            )?;
            enforce(
                WireLimit::ObjectProperties,
                limits.max_object_properties(),
                object.len(),
            )?;
            for (key, child) in object {
                enforce(WireLimit::StringBytes, limits.max_string_bytes(), key.len())?;
                validate_subtree(child, limits, depth + 1)?;
            }
        }
        Value::Array(array) => {
            enforce(
                WireLimit::NestingDepth,
                limits.max_nesting_depth(),
                depth + 1,
            )?;
            enforce(WireLimit::ArrayItems, limits.max_array_items(), array.len())?;
            for child in array {
                validate_subtree(child, limits, depth + 1)?;
            }
        }
        Value::String(value) => enforce(
            WireLimit::StringBytes,
            limits.max_string_bytes(),
            value.len(),
        )?,
        Value::Number(number) => enforce(
            WireLimit::NumberBytes,
            limits.max_number_bytes(),
            number.to_string().len(),
        )?,
        Value::Null | Value::Bool(_) => {}
    }
    Ok(())
}

fn validate_violations(evidence: &Value, limits: WireLimits) -> Result<(), WireLimitError> {
    if let Some(violations) = evidence.get("violations") {
        validate_named_array(violations, WireLimit::Violations, limits.max_violations())?;
    }
    Ok(())
}

fn validate_named_array(
    value: &Value,
    limit: WireLimit,
    maximum: usize,
) -> Result<(), WireLimitError> {
    let Value::Array(values) = value else {
        return Ok(());
    };
    enforce(limit, maximum, values.len())
}

fn enforce(limit: WireLimit, maximum: usize, actual: usize) -> Result<(), WireLimitError> {
    if actual > maximum {
        Err(WireLimitError::new(limit, maximum, actual))
    } else {
        Ok(())
    }
}
