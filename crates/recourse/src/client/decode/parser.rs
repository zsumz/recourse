//! JSON object parsing under explicit resource and shape budgets.

use serde_json::{Map, Value};

use super::validation::validate_shape;
use super::{DecodeError, DecodeLimit, DecodeLimits};

/// Parses one JSON object after enforcing every configured decode budget.
pub(crate) fn decode_object(
    body: &[u8],
    limits: DecodeLimits,
) -> Result<Map<String, Value>, DecodeError> {
    if body.len() > limits.max_body_bytes() {
        return Err(DecodeError::LimitExceeded {
            limit: DecodeLimit::BodyBytes,
            maximum: limits.max_body_bytes(),
            actual: body.len(),
        });
    }
    let value: Value = serde_json::from_slice(body).map_err(DecodeError::MalformedJson)?;
    validate_shape(&value, limits)?;
    value.as_object().cloned().ok_or(DecodeError::RootNotObject)
}
