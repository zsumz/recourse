//! Local contradictions that can be rejected without general schema solving.

use std::cmp::Ordering;

use serde_json::{Map, Number, Value};

use super::super::{SchemaViolation, build_validator, fail};

pub(super) fn validate(schema: &Map<String, Value>, path: &str) -> Result<(), SchemaViolation> {
    validate_numeric_interval(schema, path)?;
    validate_size_interval(schema, "minLength", "maxLength", path)?;
    validate_size_interval(schema, "minItems", "maxItems", path)?;
    validate_const(schema, path)?;
    validate_enum(schema, path)
}

fn validate_numeric_interval(
    schema: &Map<String, Value>,
    path: &str,
) -> Result<(), SchemaViolation> {
    let lower = [
        bound(schema, "minimum", false),
        bound(schema, "exclusiveMinimum", true),
    ];
    let upper = [
        bound(schema, "maximum", false),
        bound(schema, "exclusiveMaximum", true),
    ];
    for lower in lower.into_iter().flatten() {
        for upper in upper.into_iter().flatten() {
            let impossible = match compare_numbers(lower.value, upper.value) {
                Some(Ordering::Greater) => true,
                Some(Ordering::Equal) => lower.exclusive || upper.exclusive,
                Some(Ordering::Less) | None => false,
            };
            if impossible {
                return fail(
                    path,
                    "numeric lower bound leaves no value below the upper bound",
                );
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Bound<'a> {
    value: &'a Number,
    exclusive: bool,
}

fn bound<'a>(schema: &'a Map<String, Value>, keyword: &str, exclusive: bool) -> Option<Bound<'a>> {
    schema
        .get(keyword)
        .and_then(Value::as_number)
        .map(|value| Bound { value, exclusive })
}

fn compare_numbers(left: &Number, right: &Number) -> Option<Ordering> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return Some(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return Some(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_u64()) {
        return if left < 0 {
            Some(Ordering::Less)
        } else {
            u64::try_from(left).ok().map(|left| left.cmp(&right))
        };
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_i64()) {
        return if right < 0 {
            Some(Ordering::Greater)
        } else {
            u64::try_from(right).ok().map(|right| left.cmp(&right))
        };
    }
    left.as_f64()?.partial_cmp(&right.as_f64()?)
}

fn validate_size_interval(
    schema: &Map<String, Value>,
    minimum: &str,
    maximum: &str,
    path: &str,
) -> Result<(), SchemaViolation> {
    let Some(lower) = schema.get(minimum).and_then(Value::as_u64) else {
        return Ok(());
    };
    let Some(upper) = schema.get(maximum).and_then(Value::as_u64) else {
        return Ok(());
    };
    if lower > upper {
        fail(
            path,
            &format!("{minimum} {lower} exceeds {maximum} {upper}"),
        )
    } else {
        Ok(())
    }
}

fn validate_const(schema: &Map<String, Value>, path: &str) -> Result<(), SchemaViolation> {
    let Some(value) = schema.get("const") else {
        return Ok(());
    };
    let Some(validator) = validator_without(schema, "const") else {
        return Ok(());
    };
    if validator.is_valid(value) {
        Ok(())
    } else {
        fail(
            &format!("{path}/const"),
            "constant does not satisfy its enclosing schema",
        )
    }
}

fn validate_enum(schema: &Map<String, Value>, path: &str) -> Result<(), SchemaViolation> {
    let Some(values) = schema.get("enum").and_then(Value::as_array) else {
        return Ok(());
    };
    let Some(validator) = validator_without(schema, "enum") else {
        return Ok(());
    };
    if values.iter().any(|value| validator.is_valid(value)) {
        Ok(())
    } else {
        fail(
            &format!("{path}/enum"),
            "no enum member satisfies its enclosing schema",
        )
    }
}

fn validator_without(schema: &Map<String, Value>, keyword: &str) -> Option<jsonschema::Validator> {
    let mut constraints = schema.clone();
    constraints.remove(keyword);
    build_validator(&Value::Object(constraints)).ok()
}
