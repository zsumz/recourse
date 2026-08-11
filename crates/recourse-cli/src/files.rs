//! Bounded artifact reads and replace-after-encode lock writes.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use recourse::catalog::{
    CatalogArtifact, CatalogLock, MAX_CATALOG_ARTIFACT_BYTES, MAX_CATALOG_LOCK_BYTES,
};

use crate::documentation::GeneratedDocumentation;
use crate::error::CommandError;

const DOCS_MANIFEST: &str = ".recourse-generated";

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

pub(crate) fn write_documentation(
    out: &Path,
    documentation: &GeneratedDocumentation,
) -> Result<(), CommandError> {
    fs::create_dir_all(out).map_err(|source| CommandError::Write {
        path: out.to_owned(),
        source,
    })?;
    remove_stale_pages(out, documentation)?;
    for (relative, body) in documentation.pages() {
        let path = out.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| CommandError::Write {
                path: parent.to_owned(),
                source,
            })?;
        }
        fs::write(&path, body).map_err(|source| CommandError::Write { path, source })?;
    }
    write_manifest(out, documentation)
}

fn remove_stale_pages(
    out: &Path,
    documentation: &GeneratedDocumentation,
) -> Result<(), CommandError> {
    let manifest = out.join(DOCS_MANIFEST);
    let body = match fs::read_to_string(&manifest) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(CommandError::Read {
                path: manifest,
                source,
            });
        }
    };
    for entry in body.lines().filter(|line| !line.is_empty()) {
        let relative = validated_manifest_path(&manifest, entry)?;
        if !documentation.pages().contains_key(&relative) {
            remove_page(&out.join(relative))?;
        }
    }
    Ok(())
}

fn validated_manifest_path(manifest: &Path, entry: &str) -> Result<PathBuf, CommandError> {
    let path = PathBuf::from(entry);
    let safe = path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
        && matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("md")
        );
    if safe {
        Ok(path)
    } else {
        Err(CommandError::InvalidManifest {
            path: manifest.to_owned(),
            entry: entry.to_owned(),
        })
    }
}

fn remove_page(path: &Path) -> Result<(), CommandError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CommandError::Write {
            path: path.to_owned(),
            source,
        }),
    }
}

fn write_manifest(out: &Path, documentation: &GeneratedDocumentation) -> Result<(), CommandError> {
    let mut body = String::new();
    for path in documentation.pages().keys() {
        let Some(path) = path.to_str() else {
            continue;
        };
        body.push_str(path);
        body.push('\n');
    }
    let path = out.join(DOCS_MANIFEST);
    fs::write(&path, body).map_err(|source| CommandError::Write { path, source })
}
