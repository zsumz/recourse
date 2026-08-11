//! Bounded catalog I/O and transactional generated documentation.

mod documentation;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use atomic_write_file::AtomicWriteFile;
use recourse::catalog::{
    CatalogArtifact, CatalogLock, MAX_CATALOG_ARTIFACT_BYTES, MAX_CATALOG_LOCK_BYTES,
};

use crate::error::CommandError;
pub(crate) use documentation::write as write_documentation;

pub(crate) fn read_artifact(path: &Path) -> Result<CatalogArtifact, CommandError> {
    let body = read_bounded(path, MAX_CATALOG_ARTIFACT_BYTES)?;
    CatalogArtifact::from_slice(&body).map_err(|source| CommandError::ParseArtifact {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn read_lock(path: &Path) -> Result<CatalogLock, CommandError> {
    let body = read_bounded(path, MAX_CATALOG_LOCK_BYTES)?;
    CatalogLock::from_slice(&body).map_err(|source| CommandError::ParseLock {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn read_optional_lock(path: &Path) -> Result<Option<CatalogLock>, CommandError> {
    match read_bounded(path, MAX_CATALOG_LOCK_BYTES) {
        Ok(body) => {
            CatalogLock::from_slice(&body)
                .map(Some)
                .map_err(|source| CommandError::ParseLock {
                    path: path.to_owned(),
                    source,
                })
        }
        Err(CommandError::Read { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn write_lock(path: &Path, lock: &CatalogLock) -> Result<(), CommandError> {
    let mut body = Vec::new();
    lock.write_pretty(&mut body)
        .map_err(CommandError::EncodeLock)?;
    if body.len() > MAX_CATALOG_LOCK_BYTES {
        return Err(CommandError::EncodedLockTooLarge {
            path: path.to_owned(),
            maximum: MAX_CATALOG_LOCK_BYTES,
        });
    }
    let parsed = CatalogLock::from_slice(&body).map_err(|source| CommandError::ParseLock {
        path: path.to_owned(),
        source,
    })?;
    if &parsed != lock {
        return Err(CommandError::LockRoundTrip {
            path: path.to_owned(),
        });
    }
    atomic_replace_with(path, &body, Write::write_all)
}

pub(crate) fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, CommandError> {
    let file = File::open(path).map_err(|source| CommandError::Read {
        path: path.to_owned(),
        source,
    })?;
    let mut body = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|source| CommandError::Read {
            path: path.to_owned(),
            source,
        })?;
    if body.len() > maximum {
        return Err(CommandError::InputTooLarge {
            path: path.to_owned(),
            maximum,
        });
    }
    Ok(body)
}

pub(crate) fn atomic_replace_with(
    path: &Path,
    body: &[u8],
    write_body: impl FnOnce(&mut AtomicWriteFile, &[u8]) -> std::io::Result<()>,
) -> Result<(), CommandError> {
    let mut file = AtomicWriteFile::open(path).map_err(|source| CommandError::Write {
        path: path.to_owned(),
        source,
    })?;
    write_body(&mut file, body).map_err(|source| CommandError::Write {
        path: path.to_owned(),
        source,
    })?;
    file.flush().map_err(|source| CommandError::Write {
        path: path.to_owned(),
        source,
    })?;
    file.sync_all().map_err(|source| CommandError::Write {
        path: path.to_owned(),
        source,
    })?;
    file.commit().map_err(|source| CommandError::Write {
        path: path.to_owned(),
        source,
    })
}
