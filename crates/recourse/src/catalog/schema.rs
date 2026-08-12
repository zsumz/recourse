//! Deterministic evidence-schema normalization and conservative validation.

mod compile;
mod format;
mod resource;
mod traversal;

use schemars::SchemaGenerator;
use serde_json::{Map, Value};

use crate::diagnostic::PublicEvidence;

pub(crate) use compile::build_validator;
pub use format::{SUPPORTED_SCHEMA_FORMATS, SUPPORTED_SCHEMA_NUMERIC_FORMATS};

const ANNOTATIONS: &[&str] = &[
    "$schema",
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "readOnly",
    "writeOnly",
];

const KEYWORDS: &[&str] = &[
    "$defs",
    "$ref",
    "additionalProperties",
    "anyOf",
    "const",
    "enum",
    "exclusiveMaximum",
    "exclusiveMinimum",
    "format",
    "items",
    "maxItems",
    "maxLength",
    "maximum",
    "minItems",
    "minLength",
    "minimum",
    "multipleOf",
    "oneOf",
    "pattern",
    "properties",
    "required",
    "type",
    "uniqueItems",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SchemaViolation {
    pub(crate) path: String,
    pub(crate) reason: String,
}

pub(crate) fn normalize<E: PublicEvidence>() -> Result<Value, SchemaViolation> {
    let mut schema = SchemaGenerator::default()
        .into_root_schema_for::<E>()
        .to_value();
    compile::validate_draft(&schema)?;
    let mut references = Vec::new();
    visit_schema(&mut schema, "$", true, &mut references)?;
    resource::validate(&schema)?;
    validate_references(&schema, references)?;
    build_validator(&schema)?;
    schema.sort_all_objects();
    Ok(schema)
}

pub(crate) fn validate_artifact(schema: &mut Value) -> Result<(), SchemaViolation> {
    compile::validate_draft(schema)?;
    let mut references = Vec::new();
    visit_schema(schema, "$", true, &mut references)?;
    resource::validate(schema)?;
    validate_references(schema, references)?;
    build_validator(schema)?;
    schema.sort_all_objects();
    Ok(())
}

fn visit_schema(
    schema: &mut Value,
    path: &str,
    root: bool,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    let Some(object) = schema.as_object_mut() else {
        return fail(path, "boolean schemas are outside the Recourse profile");
    };
    for annotation in ANNOTATIONS {
        object.remove(*annotation);
    }
    reject_unknown_keywords(object, path)?;
    validate_shape(object, path, root)?;
    traversal::visit_children(object, path, root, references)
}

fn reject_unknown_keywords(object: &Map<String, Value>, path: &str) -> Result<(), SchemaViolation> {
    if let Some(keyword) = object.keys().find(|key| !KEYWORDS.contains(&key.as_str())) {
        fail(path, &format!("unsupported keyword {keyword:?}"))
    } else {
        Ok(())
    }
}

fn validate_shape(
    object: &mut Map<String, Value>,
    path: &str,
    root: bool,
) -> Result<(), SchemaViolation> {
    if root && object.get("type").and_then(Value::as_str) != Some("object") {
        return fail(path, "public evidence must have an object root");
    }
    validate_type(object.get("type"), path)?;
    format::validate(object.get("format"), object.get("type"), path)?;
    validate_string(object.get("pattern"), path, "pattern")?;
    validate_string_array(object.get_mut("required"), path, "required")?;
    validate_scalar_array(object.get_mut("enum"), path)?;
    if object.get("const").is_some_and(Value::is_object) {
        return fail(path, "object constants are outside the Recourse profile");
    }
    Ok(())
}

fn validate_type(value: Option<&Value>, path: &str) -> Result<(), SchemaViolation> {
    let Some(value) = value else {
        return Ok(());
    };
    let valid = match value {
        Value::String(kind) => supported_type(kind),
        Value::Array(kinds) => {
            !kinds.is_empty()
                && kinds
                    .iter()
                    .all(|kind| kind.as_str().is_some_and(supported_type))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        fail(path, "type must contain supported JSON primitive names")
    }
}

fn supported_type(kind: &str) -> bool {
    matches!(
        kind,
        "array" | "boolean" | "integer" | "null" | "number" | "object" | "string"
    )
}

fn validate_string(value: Option<&Value>, path: &str, name: &str) -> Result<(), SchemaViolation> {
    if value.is_none_or(Value::is_string) {
        Ok(())
    } else {
        fail(path, &format!("{name} must be a string"))
    }
}

fn validate_string_array(
    value: Option<&mut Value>,
    path: &str,
    name: &str,
) -> Result<(), SchemaViolation> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(values) = value.as_array_mut() else {
        return fail(path, &format!("{name} must be an array"));
    };
    if values.iter().any(|value| !value.is_string()) {
        return fail(path, &format!("{name} must contain only strings"));
    }
    values.sort_by_key(Value::to_string);
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return fail(path, &format!("{name} contains a duplicate"));
    }
    Ok(())
}

fn validate_scalar_array(value: Option<&mut Value>, path: &str) -> Result<(), SchemaViolation> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(values) = value.as_array_mut() else {
        return fail(path, "enum must be an array");
    };
    if values.is_empty()
        || values.iter().any(Value::is_array)
        || values.iter().any(Value::is_object)
    {
        return fail(path, "enum must be a finite nonempty set of scalar values");
    }
    values.sort_by_key(Value::to_string);
    Ok(())
}

fn validate_references(
    schema: &Value,
    references: Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    for (path, reference) in references {
        let pointer = reference.trim_start_matches('#');
        if schema.pointer(pointer).is_none() {
            return fail(&path, &format!("unresolved local reference {reference:?}"));
        }
    }
    Ok(())
}

pub(super) fn fail<T>(path: &str, reason: &str) -> Result<T, SchemaViolation> {
    Err(SchemaViolation {
        path: path.to_owned(),
        reason: reason.to_owned(),
    })
}
