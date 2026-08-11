//! Complete staged generation before documentation-tree replacement.

mod tree;

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{documentation::GeneratedDocumentation, error::CommandError};

use self::tree::StagingTree;

const MANIFEST: &str = ".recourse-generated";

pub(crate) fn write(
    out: &Path,
    documentation: &GeneratedDocumentation,
) -> Result<(), CommandError> {
    validate_generated_paths(documentation)?;
    let staging = StagingTree::new(out)?;
    staging.copy_existing(out)?;
    remove_stale_pages(staging.path(), documentation)?;
    write_pages(staging.path(), documentation)?;
    write_manifest(staging.path(), documentation)?;
    validate_staging(staging.path(), documentation)?;
    staging.commit(out)
}

fn validate_generated_paths(documentation: &GeneratedDocumentation) -> Result<(), CommandError> {
    for path in documentation.pages().keys() {
        if !valid_page_path(path) {
            return Err(CommandError::UnsafeDocumentation {
                path: path.clone(),
                reason: "generated page path must be a relative Markdown path",
            });
        }
    }
    Ok(())
}

fn remove_stale_pages(
    staging: &Path,
    documentation: &GeneratedDocumentation,
) -> Result<(), CommandError> {
    let manifest = staging.join(MANIFEST);
    let body = match fs::read_to_string(&manifest) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(read_error(&manifest, source)),
    };
    for entry in body.lines().filter(|line| !line.is_empty()) {
        let relative = validated_manifest_path(&manifest, entry)?;
        if !documentation.pages().contains_key(&relative) {
            let path = staging.join(relative);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(write_error(&path, source)),
            }
        }
    }
    Ok(())
}

fn write_pages(staging: &Path, documentation: &GeneratedDocumentation) -> Result<(), CommandError> {
    for (relative, body) in documentation.pages() {
        let path = staging.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| write_error(parent, source))?;
        }
        fs::write(&path, body).map_err(|source| write_error(&path, source))?;
    }
    Ok(())
}

fn write_manifest(
    staging: &Path,
    documentation: &GeneratedDocumentation,
) -> Result<(), CommandError> {
    let body = documentation
        .pages()
        .keys()
        .filter_map(|path| path.to_str())
        .fold(String::new(), |mut body, path| {
            body.push_str(path);
            body.push('\n');
            body
        });
    let path = staging.join(MANIFEST);
    fs::write(&path, body).map_err(|source| write_error(&path, source))
}

fn validate_staging(
    staging: &Path,
    documentation: &GeneratedDocumentation,
) -> Result<(), CommandError> {
    for (relative, expected) in documentation.pages() {
        let path = staging.join(relative);
        let actual = fs::read_to_string(&path).map_err(|source| read_error(&path, source))?;
        if &actual != expected {
            return Err(CommandError::UnsafeDocumentation {
                path,
                reason: "staged page did not match the rendered content",
            });
        }
    }
    Ok(())
}

fn validated_manifest_path(manifest: &Path, entry: &str) -> Result<PathBuf, CommandError> {
    let path = PathBuf::from(entry);
    if valid_page_path(&path) {
        Ok(path)
    } else {
        Err(CommandError::InvalidManifest {
            path: manifest.to_owned(),
            entry: entry.to_owned(),
        })
    }
}

fn valid_page_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
        && path.extension().and_then(|value| value.to_str()) == Some("md")
}

fn read_error(path: &Path, source: std::io::Error) -> CommandError {
    CommandError::Read {
        path: path.to_owned(),
        source,
    }
}

fn write_error(path: &Path, source: std::io::Error) -> CommandError {
    CommandError::Write {
        path: path.to_owned(),
        source,
    }
}
