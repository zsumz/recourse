//! Mandatory container-depth closure against the default wire envelope.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::super::{SchemaViolation, fail, number};
use super::exact_type;
use crate::wire::WireLimits;

pub(super) fn validate(schema: &Value) -> Result<(), SchemaViolation> {
    let depth = minimum_container_depth(schema, schema, &mut BTreeSet::new(), "$")?;
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

fn minimum_container_depth(
    schema: &Value,
    root: &Value,
    references: &mut BTreeSet<String>,
    path: &str,
) -> Result<usize, SchemaViolation> {
    let Some(object) = schema.as_object() else {
        return Ok(0);
    };
    if let Some(value) = object.get("const") {
        return Ok(value_depth(value));
    }
    if let Some(values) = object.get("enum").and_then(Value::as_array) {
        return Ok(values.iter().map(value_depth).min().unwrap_or(0));
    }
    let mut depth = type_depth(object, root, references, path)?;
    for keyword in ["anyOf", "oneOf"] {
        let mut choice = None;
        for value in object
            .get(keyword)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let child = minimum_container_depth(value, root, references, path)?;
            choice = Some(choice.map_or(child, |current: usize| current.min(child)));
        }
        depth = depth.max(choice.unwrap_or(0));
    }
    if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
        if !references.insert(reference.to_owned()) {
            return Ok(usize::MAX);
        }
        if let Some(referenced) = root.pointer(reference.trim_start_matches('#')) {
            depth = depth.max(minimum_container_depth(referenced, root, references, path)?);
        }
        references.remove(reference);
    }
    Ok(depth)
}

fn type_depth(
    schema: &Map<String, Value>,
    root: &Value,
    references: &mut BTreeSet<String>,
    path: &str,
) -> Result<usize, SchemaViolation> {
    if exact_type(schema.get("type"), "object") {
        let mut child = 0;
        for property in schema
            .get("required")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|name| schema.get("properties")?.get(name))
        {
            child = child.max(minimum_container_depth(property, root, references, path)?);
        }
        Ok(1usize.saturating_add(child))
    } else if exact_type(schema.get("type"), "array") {
        let mandatory = match schema.get("minItems").and_then(Value::as_number) {
            Some(minimum) => number::is_positive(minimum, path)?,
            None => false,
        };
        let child = if mandatory {
            match schema.get("items") {
                Some(items) => minimum_container_depth(items, root, references, path)?,
                None => 0,
            }
        } else {
            0
        };
        Ok(1usize.saturating_add(child))
    } else {
        Ok(0)
    }
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
