//! Generated schemas must fit the catalog artifact parser's resource profile.

use serde_json::Value;

use super::SchemaViolation;
use crate::{catalog::artifact::artifact_limits, wire::validate_value};

pub(super) fn validate(schema: &Value) -> Result<(), SchemaViolation> {
    validate_value(schema, artifact_limits()).map_err(|error| SchemaViolation {
        path: "$".to_owned(),
        reason: format!("schema exceeds catalog artifact limits: {error}"),
    })
}
