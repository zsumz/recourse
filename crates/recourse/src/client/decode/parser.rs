//! JSON object parsing under explicit resource and shape budgets.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::unique::UniqueValue;
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
    let mut deserializer = serde_json::Deserializer::from_slice(body);
    let value = UniqueValue::deserialize(&mut deserializer)
        .map_err(DecodeError::MalformedJson)?
        .into_inner();
    deserializer.end().map_err(DecodeError::MalformedJson)?;
    validate_shape(&value, limits)?;
    value.as_object().cloned().ok_or(DecodeError::RootNotObject)
}
