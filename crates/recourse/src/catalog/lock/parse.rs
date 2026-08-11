//! Bounded parsing and semantic validation for append-only catalog locks.

use serde_json::{Value, json};

use crate::{
    catalog::{CatalogArtifact, Code},
    client::{DecodeLimits, decode_object},
};

use super::{
    CatalogLock, LockEntry, LockParseError, MAX_CATALOG_LOCK_BYTES,
    replacement::{self, ReplacementIssue},
};

pub(super) fn parse_lock(body: &[u8]) -> Result<CatalogLock, LockParseError> {
    let limits = DecodeLimits::default()
        .with_max_body_bytes(MAX_CATALOG_LOCK_BYTES)
        .with_max_nesting_depth(64)
        .with_max_object_properties(16_384)
        .with_max_array_items(32_768)
        .with_max_string_bytes(512 * 1024);
    let object = decode_object(body, limits).map_err(LockParseError::Decode)?;
    let lock = serde_json::from_value(Value::Object(object)).map_err(LockParseError::Structure)?;
    validate(&lock)?;
    Ok(lock)
}

fn validate(lock: &CatalogLock) -> Result<(), LockParseError> {
    if lock.schema_version != 1 {
        return Err(LockParseError::UnsupportedSchemaVersion {
            found: lock.schema_version,
        });
    }
    for pair in lock.entries.windows(2) {
        if pair[0].number() >= pair[1].number() {
            return invalid("entries", "numbers must be strictly increasing");
        }
    }
    validate_definitions(lock)?;
    validate_entries(lock)?;
    validate_replacements(lock)
}

fn validate_definitions(lock: &CatalogLock) -> Result<(), LockParseError> {
    let diagnostics = lock
        .entries
        .iter()
        .filter_map(LockEntry::diagnostic)
        .collect::<Vec<_>>();
    let artifact = json!({
        "schema_version": 1,
        "catalog": &lock.catalog,
        "diagnostics": diagnostics,
        "problem_sets": {},
    });
    let body = serde_json::to_vec(&artifact).map_err(LockParseError::Structure)?;
    CatalogArtifact::from_slice(&body).map_err(|error| LockParseError::Invalid {
        path: "entries".to_owned(),
        reason: error.to_string(),
    })?;
    Ok(())
}

fn validate_entries(lock: &CatalogLock) -> Result<(), LockParseError> {
    for entry in &lock.entries {
        validate_identity(lock, entry)?;
        if let LockEntry::Retired { reason, .. } = entry
            && reason.trim().is_empty()
        {
            return invalid(&entry_path(entry), "retirement reason must be nonempty");
        }
    }
    Ok(())
}

fn validate_identity(lock: &CatalogLock, entry: &LockEntry) -> Result<(), LockParseError> {
    let code =
        Code::new(lock.prefix(), entry.number()).map_err(|error| LockParseError::Invalid {
            path: entry_path(entry),
            reason: error.to_string(),
        })?;
    if entry.code() != &code || entry.type_uri() != format!("{}{code}", lock.type_base()) {
        return invalid(
            &entry_path(entry),
            "number, code, and type must share the lock namespace",
        );
    }
    Ok(())
}

fn validate_replacements(lock: &CatalogLock) -> Result<(), LockParseError> {
    match replacement::validate(lock) {
        Ok(()) => Ok(()),
        Err(ReplacementIssue::MissingOrReserved {
            source,
            replacement,
        }) => invalid(
            &format!("entries.{source}"),
            &format!("replacement {replacement} must identify an existing active or retired entry"),
        ),
        Err(ReplacementIssue::Cycle { code }) => invalid(
            &format!("entries.{code}"),
            "replacement chains must be acyclic",
        ),
    }
}

fn entry_path(entry: &LockEntry) -> String {
    format!("entries.{}", entry.code())
}

fn invalid<T>(path: &str, reason: &str) -> Result<T, LockParseError> {
    Err(LockParseError::Invalid {
        path: path.to_owned(),
        reason: reason.to_owned(),
    })
}
