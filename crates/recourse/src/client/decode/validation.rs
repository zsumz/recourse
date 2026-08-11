//! Complete JSON-tree traversal enforcing local and semantic collection limits.

use super::{DecodeError, DecodeLimits};

pub(super) fn validate_shape(
    value: &serde_json::Value,
    limits: DecodeLimits,
) -> Result<(), DecodeError> {
    crate::wire::validate_value(value, limits).map_err(|error| DecodeError::LimitExceeded {
        limit: error.limit(),
        maximum: error.maximum(),
        actual: error.actual(),
    })
}
