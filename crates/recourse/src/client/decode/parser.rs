//! JSON object parsing under explicit resource and shape budgets.

use serde_json::{Map, Value};

use super::validation::validate_shape;
use super::{DecodeError, DecodeLimit, DecodeLimits};

/// Parses one JSON object after enforcing every configured decode budget.
pub(crate) fn decode_object(
    body: &[u8],
    limits: DecodeLimits,
) -> Result<Map<String, Value>, DecodeError> {
    let value = decode_value(body, limits, 0)?;
    validate_shape(&value, limits)?;
    into_object(value)
}

/// Parses an object that will be nested inside a diagnostic envelope.
pub(crate) fn decode_embedded_object(
    body: &[u8],
    limits: DecodeLimits,
) -> Result<Map<String, Value>, DecodeError> {
    let value = decode_value(body, limits, 1)?;
    crate::wire::validate_embedded(&value, limits).map_err(limit_error)?;
    into_object(value)
}

fn decode_value(
    body: &[u8],
    limits: DecodeLimits,
    initial_depth: usize,
) -> Result<Value, DecodeError> {
    if body.len() > limits.max_body_bytes() {
        return Err(DecodeError::LimitExceeded {
            limit: DecodeLimit::BodyBytes,
            maximum: limits.max_body_bytes(),
            actual: body.len(),
        });
    }
    super::unique::parse(body, limits, initial_depth)
}

fn into_object(value: Value) -> Result<Map<String, Value>, DecodeError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(DecodeError::RootNotObject),
    }
}

fn limit_error(error: crate::wire::WireLimitError) -> DecodeError {
    DecodeError::LimitExceeded {
        limit: error.limit(),
        maximum: error.maximum(),
        actual: error.actual(),
    }
}
