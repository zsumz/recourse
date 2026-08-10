//! Recursive traversal of child schemas in supported keyword positions.

use serde_json::{Map, Value};

use super::{SchemaViolation, fail, visit_schema};

pub(super) fn visit_children(
    object: &mut Map<String, Value>,
    path: &str,
    root: bool,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    collect_reference(object, path, references)?;
    visit_named(object, "$defs", path, references)?;
    visit_named(object, "properties", path, references)?;
    visit_single(object, "items", path, references)?;
    visit_additional_properties(object, path, root, references)?;
    visit_schema_array(object, "anyOf", path, references)?;
    visit_schema_array(object, "oneOf", path, references)
}

fn collect_reference(
    object: &Map<String, Value>,
    path: &str,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    let Some(reference) = object.get("$ref") else {
        return Ok(());
    };
    let Some(reference) = reference.as_str() else {
        return fail(path, "$ref must be a string");
    };
    if !reference.starts_with("#/$defs/") {
        return fail(path, "only local $defs references are supported");
    }
    references.push((path.to_owned(), reference.to_owned()));
    Ok(())
}

fn visit_named(
    object: &mut Map<String, Value>,
    keyword: &str,
    path: &str,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    let Some(value) = object.get_mut(keyword) else {
        return Ok(());
    };
    let Some(children) = value.as_object_mut() else {
        return fail(path, &format!("{keyword} must be an object"));
    };
    for (name, child) in children {
        let child_path = format!("{path}/{keyword}/{}", escape(name));
        visit_schema(child, &child_path, false, references)?;
    }
    Ok(())
}

fn visit_single(
    object: &mut Map<String, Value>,
    keyword: &str,
    path: &str,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    if let Some(child) = object.get_mut(keyword) {
        visit_schema(child, &format!("{path}/{keyword}"), false, references)?;
    }
    Ok(())
}

fn visit_additional_properties(
    object: &mut Map<String, Value>,
    path: &str,
    root: bool,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    let Some(value) = object.get_mut("additionalProperties") else {
        return Ok(());
    };
    if root && value != &Value::Bool(false) {
        return fail(path, "root arbitrary maps are not public evidence objects");
    }
    if value.is_object() {
        visit_schema(
            value,
            &format!("{path}/additionalProperties"),
            false,
            references,
        )
    } else if value.is_boolean() {
        Ok(())
    } else {
        fail(path, "additionalProperties must be a boolean or schema")
    }
}

fn visit_schema_array(
    object: &mut Map<String, Value>,
    keyword: &str,
    path: &str,
    references: &mut Vec<(String, String)>,
) -> Result<(), SchemaViolation> {
    let Some(value) = object.get_mut(keyword) else {
        return Ok(());
    };
    let Some(children) = value.as_array_mut() else {
        return fail(path, &format!("{keyword} must be an array"));
    };
    if children.is_empty() {
        return fail(path, &format!("{keyword} must not be empty"));
    }
    for (index, child) in children.iter_mut().enumerate() {
        visit_schema(
            child,
            &format!("{path}/{keyword}/{index}"),
            false,
            references,
        )?;
    }
    Ok(())
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
