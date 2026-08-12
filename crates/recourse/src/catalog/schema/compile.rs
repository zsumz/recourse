//! Draft validation and offline runtime-validator compilation.

use std::fmt::Display;

use serde_json::Value;

use super::SchemaViolation;

pub(super) fn validate_draft(schema: &Value) -> Result<(), SchemaViolation> {
    jsonschema::draft202012::meta::validate(schema).map_err(|error| SchemaViolation {
        path: schema_path(error.instance_path()),
        reason: format!("invalid Draft 2020-12 schema: {error}"),
    })
}

pub(crate) fn build_validator(schema: &Value) -> Result<jsonschema::Validator, SchemaViolation> {
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false)
        .build(schema)
        .map_err(|error| SchemaViolation {
            path: schema_path(error.schema_path()),
            reason: format!("schema cannot be compiled: {error}"),
        })
}

fn schema_path(path: impl Display) -> String {
    let path = path.to_string();
    if path.is_empty() {
        "$".to_owned()
    } else {
        format!("${path}")
    }
}
