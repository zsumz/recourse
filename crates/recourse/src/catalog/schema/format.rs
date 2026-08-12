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

pub(super) fn validate(value: Option<&Value>, path: &str) -> Result<(), SchemaViolation> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some(format) = value.as_str() else {
        return fail(path, "format must be a string");
    };
    if SUPPORTED_SCHEMA_FORMATS.contains(&format) {
        Ok(())
    } else {
        fail(path, &format!("unsupported format {format:?}"))
    }
}
