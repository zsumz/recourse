//! Closed JSON Schema format vocabulary enforced as runtime assertions.

use serde_json::Value;

use super::{SchemaViolation, fail};

/// JSON Schema formats enforced as assertions by the Recourse evidence profile.
pub const SUPPORTED_SCHEMA_FORMATS: &[&str] = &[
    "date",
    "date-time",
    "duration",
    "email",
    "hostname",
    "idn-email",
    "idn-hostname",
    "ip",
    "ipv4",
    "ipv6",
    "iri",
    "iri-reference",
    "json-pointer",
    "regex",
    "relative-json-pointer",
    "time",
    "uri",
    "uri-reference",
    "uri-template",
    "uuid",
];

/// Schemars numeric representations retained beside enforced JSON types and bounds.
pub const SUPPORTED_SCHEMA_NUMERIC_FORMATS: &[&str] = &[
    "double", "float", "int", "int8", "int16", "int32", "int64", "int128", "uint", "uint8",
    "uint16", "uint32", "uint64", "uint128",
];

pub(super) fn validate(
    value: Option<&Value>,
    schema_type: Option<&Value>,
    path: &str,
) -> Result<(), SchemaViolation> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(format) = value.as_str() else {
        return fail(path, "format must be a string");
    };
    let supported_string =
        SUPPORTED_SCHEMA_FORMATS.contains(&format) && has_type(schema_type, "string");
    let supported_numeric = SUPPORTED_SCHEMA_NUMERIC_FORMATS.contains(&format)
        && (has_type(schema_type, "integer") || has_type(schema_type, "number"));
    if supported_string || supported_numeric {
        Ok(())
    } else {
        fail(path, &format!("unsupported format {format:?}"))
    }
}

fn has_type(value: Option<&Value>, expected: &str) -> bool {
    match value {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some(expected)),
        _ => false,
    }
}
