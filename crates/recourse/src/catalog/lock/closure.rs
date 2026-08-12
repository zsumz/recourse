//! Public lock mutations commit only parser-closed candidate states.

use super::{CatalogLock, write};

pub(super) fn validate(candidate: &CatalogLock) -> Result<(), String> {
    let body = write::pretty(candidate).map_err(|error| error.to_string())?;
    let parsed = CatalogLock::from_slice(&body).map_err(|error| error.to_string())?;
    if &parsed == candidate {
        Ok(())
    } else {
        Err("encoded lock did not round-trip with semantic equality".to_owned())
    }
}
