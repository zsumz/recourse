//! Numbers that governed Rust evidence serializers can emit exactly.

use std::cmp::Ordering;

use serde_json::{Number, Value};

use super::{SchemaViolation, compare, validate_token};

pub(crate) fn value_is_public(value: &Value, path: &str) -> Result<bool, SchemaViolation> {
    match value {
        Value::Number(number) => is_public(number, path),
        Value::Array(values) => values_are_public(values.iter(), path),
        Value::Object(object) => values_are_public(object.values(), path),
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(true),
    }
}

fn values_are_public<'a>(
    values: impl Iterator<Item = &'a Value>,
    path: &str,
) -> Result<bool, SchemaViolation> {
    for value in values {
        if !value_is_public(value, path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn is_public(number: &Number, path: &str) -> Result<bool, SchemaViolation> {
    validate_token(number, path)?;
    if number.as_i64().is_some() || number.as_u64().is_some() {
        return Ok(true);
    }
    if emitted_float_matches::<f32>(number, path)? {
        return Ok(true);
    }
    emitted_float_matches::<f64>(number, path)
}

pub(crate) fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            compare(left, right, "$").is_ok_and(|ordering| ordering == Ordering::Equal)
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| values_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, left)| {
                    right
                        .get(key)
                        .is_some_and(|right| values_equal(left, right))
                })
        }
        _ => left == right,
    }
}

pub(crate) fn unordered_values_equal(left: &[Value], right: &[Value]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut matched = vec![false; right.len()];
    for left in left {
        let Some(index) = right
            .iter()
            .enumerate()
            .position(|(index, right)| !matched[index] && values_equal(left, right))
        else {
            return false;
        };
        matched[index] = true;
    }
    true
}

fn emitted_float_matches<T>(number: &Number, path: &str) -> Result<bool, SchemaViolation>
where
    T: std::str::FromStr + serde::Serialize + Copy + Float,
{
    let Ok(value) = number.as_str().parse::<T>() else {
        return Ok(false);
    };
    if !value.finite() {
        return Ok(false);
    }
    let encoded = serde_json::to_string(&value).map_err(|error| SchemaViolation {
        path: path.to_owned(),
        reason: format!("public numeric emission failed: {error}"),
    })?;
    let emitted = serde_json::from_str::<Number>(&encoded).map_err(|error| SchemaViolation {
        path: path.to_owned(),
        reason: format!("public numeric emission was not JSON: {error}"),
    })?;
    Ok(compare(number, &emitted, path)? == Ordering::Equal)
}

trait Float {
    fn finite(self) -> bool;
}

impl Float for f32 {
    fn finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl Float for f64 {
    fn finite(self) -> bool {
        f64::is_finite(self)
    }
}
