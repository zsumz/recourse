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
    let value = decode_value(body, limits)?;
    validate_shape(&value, limits)?;
    into_object(value)
}

/// Parses an object that will be nested inside a diagnostic envelope.
pub(crate) fn decode_embedded_object(
    body: &[u8],
    limits: DecodeLimits,
) -> Result<Map<String, Value>, DecodeError> {
    let value = decode_value(body, limits)?;
    crate::wire::validate_embedded(&value, limits).map_err(limit_error)?;
    into_object(value)
}

fn decode_value(body: &[u8], limits: DecodeLimits) -> Result<Value, DecodeError> {
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
    Ok(value)
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
