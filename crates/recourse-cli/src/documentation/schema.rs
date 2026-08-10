//! Evidence and impact field tables plus exact normalized JSON Schema.

use std::{collections::BTreeSet, fmt::Write as _};

use serde_json::Value;

use super::markdown;

pub(super) fn section(title: &str, schema: &Value) -> Result<String, serde_json::Error> {
    let mut body = format!("## {title}\n\n");
    push_fields(&mut body, schema);
    let _ = write!(body, "\n### {title} JSON Schema\n\n");
    let json = serde_json::to_string_pretty(schema)?;
    for line in json.lines() {
        body.push_str("    ");
        body.push_str(line);
        body.push('\n');
    }
    body.push('\n');
    Ok(body)
}

fn push_fields(body: &mut String, schema: &Value) {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        body.push_str("_No public fields._\n");
        return;
    };
    if properties.is_empty() {
        body.push_str("_No public fields._\n");
        return;
    }
    let required = required_fields(schema);
    body.push_str("| Field | Required | Shape |\n|---|---:|---|\n");
    for (name, value) in properties {
        let requirement = if required.contains(name.as_str()) {
            "yes"
        } else {
            "no"
        };
        let _ = writeln!(
            body,
            "| {} | {requirement} | {} |",
            markdown::code(name),
            markdown::table_cell(&shape(value))
        );
    }
}

fn required_fields(schema: &Value) -> BTreeSet<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn shape(schema: &Value) -> String {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        return format!("reference {reference}");
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return format!("enum with {} values", values.len());
    }
    match schema.get("type") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" or "),
        _ if schema.get("oneOf").is_some() => "union".to_owned(),
        _ => "object".to_owned(),
    }
}
