//! Bounded, duplicate-aware materialization of custom public serializers.

use serde::Serialize;
use serde_json::Value;

use crate::{
    client::{DecodeError, decode_embedded_object},
    wire::{BoundedJsonError, WireLimitError, WireLimits, to_bounded_vec},
};

pub(crate) enum MaterializeError {
    Json(serde_json::Error),
    NotObject,
    Limit(WireLimitError),
}

pub(crate) fn object<T: Serialize>(
    value: &T,
    limits: WireLimits,
) -> Result<Value, MaterializeError> {
    let body = to_bounded_vec(value, limits).map_err(|error| match error {
        BoundedJsonError::Serialize(error) => MaterializeError::Json(error),
        BoundedJsonError::Limit(error) => MaterializeError::Limit(error),
    })?;
    let object = decode_embedded_object(&body, limits).map_err(|error| match error {
        DecodeError::MalformedJson(error) => MaterializeError::Json(error),
        DecodeError::RootNotObject => MaterializeError::NotObject,
        DecodeError::LimitExceeded {
            limit,
            maximum,
            actual,
        } => MaterializeError::Limit(WireLimitError::new(limit, maximum, actual)),
    })?;
    let mut value = Value::Object(object);
    value.sort_all_objects();
    Ok(value)
}
