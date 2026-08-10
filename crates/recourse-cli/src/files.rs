//! Bounded artifact reads and replace-after-encode lock writes.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use recourse::catalog::{CatalogArtifact, CatalogLock};

use crate::documentation::GeneratedDocumentation;
use crate::error::CommandError;

const DOCS_MANIFEST: &str = ".recourse-generated";

pub(crate) fn read_artifact(path: &Path) -> Result<CatalogArtifact, CommandError> {
    let body = read(path)?;
    CatalogArtifact::from_slice(&body).map_err(|source| CommandError::ParseArtifact {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn read_lock(path: &Path) -> Result<CatalogLock, CommandError> {
    let body = read(path)?;
    CatalogLock::from_slice(&body).map_err(|source| CommandError::ParseLock {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn read_optional_lock(path: &Path) -> Result<Option<CatalogLock>, CommandError> {
    match fs::read(path) {
        Ok(body) => {
            CatalogLock::from_slice(&body)
                .map(Some)
                .map_err(|source| CommandError::ParseLock {
                    path: path.to_owned(),
                    source,
                })
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(CommandError::Read {
            path: path.to_owned(),
            source,
        }),
    }
}

pub(crate) fn write_lock(path: &Path, lock: &CatalogLock) -> Result<(), CommandError> {
    let mut body = Vec::new();
    lock.write_pretty(&mut body)
        .map_err(CommandError::EncodeLock)?;
    fs::write(path, body).map_err(|source| CommandError::Write {
        path: path.to_owned(),
        source,
    })
}

fn read(path: &Path) -> Result<Vec<u8>, CommandError> {
    fs::read(path).map_err(|source| CommandError::Read {
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
