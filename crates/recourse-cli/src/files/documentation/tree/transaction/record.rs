//! Lossless output identity and strict transaction-journal validation.

use std::{
    ffi::OsString,
    fs, io,
    path::{Component, Path, PathBuf},
};

use crate::error::CommandError;

use super::super::{read_error, unsafe_path};

const JOURNAL_PREFIX: &str = ".recourse-transaction-";
const BACKUP_PREFIX: &str = ".recourse-backup-";
const FORMAT: &str = "recourse-documentation-transaction-v1";

pub(super) fn backup_name(out: &Path, journal: &Path, body: &[u8]) -> Result<String, CommandError> {
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| unsafe_path(journal, "documentation transaction journal is invalid"))?;
    if value.get("format").and_then(serde_json::Value::as_str) != Some(FORMAT)
        || value.get("output").and_then(serde_json::Value::as_str)
            != Some(&encode_path(&canonical_output(out)?))
    {
        return Err(unsafe_path(
            journal,
            "documentation transaction does not own this output",
        ));
    }
    let name = value
        .get("backup")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| unsafe_path(journal, "documentation transaction backup is invalid"))?;
    if !valid_backup_name(name) {
        return Err(unsafe_path(
            journal,
            "documentation transaction backup is invalid",
        ));
    }
    Ok(name.to_owned())
}

pub(super) fn canonical_output(out: &Path) -> Result<PathBuf, CommandError> {
    let name = out
        .file_name()
        .ok_or_else(|| unsafe_path(out, "output must name a directory"))?;
    let canonical_parent =
        fs::canonicalize(parent(out)).map_err(|source| read_error(parent(out), source))?;
    Ok(canonical_parent.join(name))
}

pub(super) fn journal_body(
    canonical_output: &Path,
    backup_name: &str,
) -> Result<Vec<u8>, CommandError> {
    serde_json::to_vec(&serde_json::json!({
        "format": FORMAT,
        "output": encode_path(canonical_output),
        "backup": backup_name,
    }))
    .map_err(CommandError::from)
}

pub(super) fn journal_path(out: &Path) -> Result<PathBuf, CommandError> {
    let name = out
        .file_name()
        .ok_or_else(|| unsafe_path(out, "output must name a directory"))?;
    let mut journal_name = OsString::from(JOURNAL_PREFIX);
    journal_name.push(name);
    Ok(out.with_file_name(journal_name))
}

pub(super) fn parent(out: &Path) -> &Path {
    out.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

pub(super) fn reject_legacy_backup(out: &Path) -> Result<(), CommandError> {
    let name = out
        .file_name()
        .ok_or_else(|| unsafe_path(out, "output must name a directory"))?;
    let mut legacy_name = OsString::from(BACKUP_PREFIX);
    legacy_name.push(name);
    let legacy = out.with_file_name(legacy_name);
    match fs::symlink_metadata(&legacy) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(unsafe_path(
            &legacy,
            "unowned legacy recovery backup already exists",
        )),
        Err(source) => Err(read_error(&legacy, source)),
    }
}

fn valid_backup_name(name: &str) -> bool {
    name.starts_with(BACKUP_PREFIX)
        && Path::new(name).components().count() == 1
        && matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
}

#[cfg(unix)]
fn encode_path(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    hex(path.as_os_str().as_bytes(), "unix:")
}

#[cfg(windows)]
fn encode_path(path: &Path) -> String {
    use std::os::windows::ffi::OsStrExt;
    let mut encoded = String::from("windows:");
    for value in path.as_os_str().encode_wide() {
        push_hex(&mut encoded, usize::from(value), 4);
    }
    encoded
}

#[cfg(not(any(unix, windows)))]
fn encode_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(unix)]
fn hex(bytes: &[u8], prefix: &str) -> String {
    let mut encoded = String::with_capacity(prefix.len() + bytes.len() * 2);
    encoded.push_str(prefix);
    for byte in bytes {
        push_hex(&mut encoded, usize::from(*byte), 2);
    }
    encoded
}

#[cfg(any(unix, windows))]
fn push_hex(encoded: &mut String, value: usize, width: usize) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for offset in (0..width).rev() {
        encoded.push(char::from(DIGITS[(value >> (offset * 4)) & 0x0f]));
    }
}
