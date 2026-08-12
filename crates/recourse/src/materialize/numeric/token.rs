//! Fail-closed handling for serializer-private token names.

use serde::ser::Error;

const SERDE_JSON_PRIVATE_PREFIX: &str = "$serde_json::private::";

pub(super) fn reject<E: Error>(name: &str) -> Result<(), E> {
    if name.starts_with(SERDE_JSON_PRIVATE_PREFIX) {
        Err(E::custom(
            "raw JSON serializer tokens are outside the governed evidence profile",
        ))
    } else {
        Ok(())
    }
}
