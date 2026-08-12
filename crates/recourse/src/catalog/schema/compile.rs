//! Draft validation and offline runtime-validator compilation.

use std::fmt::Display;

use serde_json::Value;

use super::{SchemaViolation, format::SUPPORTED_SCHEMA_NUMERIC_FORMATS};

pub(super) fn validate_draft(schema: &Value) -> Result<(), SchemaViolation> {
    jsonschema::draft202012::meta::validate(schema).map_err(|error| SchemaViolation {
        path: schema_path(error.instance_path()),
        reason: format!("invalid Draft 2020-12 schema: {error}"),
    })
}

pub(crate) fn build_validator(schema: &Value) -> Result<jsonschema::Validator, SchemaViolation> {
    let mut options = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .should_ignore_unknown_formats(false);
    for format in SUPPORTED_SCHEMA_NUMERIC_FORMATS {
        options = options.with_format(*format, numeric_representation);
    }
    options
        .with_format("ip", valid_ip)
        .build(schema)
        .map_err(|error| SchemaViolation {
            path: schema_path(error.schema_path()),
            reason: format!("schema cannot be compiled: {error}"),
        })
}

fn numeric_representation(_value: &str) -> bool {
    true
}

fn valid_ip(value: &str) -> bool {
    value.parse::<std::net::IpAddr>().is_ok()
}

fn schema_path(path: impl Display) -> String {
    let path = path.to_string();
    if path.is_empty() {
        "$".to_owned()
    } else {
        format!("${path}")
    }
}
