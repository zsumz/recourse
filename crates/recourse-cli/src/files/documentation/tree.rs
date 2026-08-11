//! Symlink-safe copying, staging cleanup, and output-tree commit.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::error::CommandError;

pub(super) struct StagingTree {
    path: PathBuf,
}

impl StagingTree {
    pub(super) fn new(out: &Path) -> Result<Self, CommandError> {
        if out.file_name().is_none() {
            return Err(unsafe_path(out, "output must name a directory"));
        }
        let parent = out
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| write_error(parent, source))?;
        reject_root_symlink(out)?;
        for attempt in 0..128_u8 {
            let path = sibling(parent, "stage", attempt);
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(write_error(&path, source)),
            }
        }
        Err(write_error(
            parent,
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "no staging name was available",
            ),
        ))
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn copy_existing(&self, out: &Path) -> Result<(), CommandError> {
        match fs::symlink_metadata(out) {
            Ok(metadata) if metadata.is_dir() => copy_directory(out, &self.path),
            Ok(_) => Err(unsafe_path(out, "output exists but is not a directory")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(read_error(out, source)),
        }
    }

    pub(super) fn commit(self, out: &Path) -> Result<(), CommandError> {
        if !out.exists() {
            return fs::rename(&self.path, out).map_err(|source| write_error(out, source));
        }
        let parent = out
            .parent()
            .ok_or_else(|| unsafe_path(out, "output has no parent"))?;
        let backup = available_sibling(parent, "backup")?;
        fs::rename(out, &backup).map_err(|source| write_error(out, source))?;
        if let Err(source) = fs::rename(&self.path, out) {
            let rollback = fs::rename(&backup, out);
            return Err(match rollback {
                Ok(()) => write_error(out, source),
                Err(rollback) => write_error(
                    out,
                    io::Error::other(format!(
                        "commit failed: {source}; rollback failed: {rollback}"
                    )),
                ),
            });
        }
        fs::remove_dir_all(&backup).map_err(|source| write_error(&backup, source))
    }
}

impl Drop for StagingTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), CommandError> {
    for entry in fs::read_dir(source).map_err(|error| read_error(source, error))? {
        let entry = entry.map_err(|error| read_error(source, error))?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let kind = entry
            .file_type()
            .map_err(|error| read_error(&from, error))?;
        if kind.is_symlink() {
            return Err(unsafe_path(
                &from,
                "symlinks are not allowed in documentation output",
            ));
        }
        if kind.is_dir() {
            fs::create_dir(&to).map_err(|error| write_error(&to, error))?;
            copy_directory(&from, &to)?;
        } else if kind.is_file() {
            fs::copy(&from, &to).map_err(|error| write_error(&to, error))?;
        } else {
            return Err(unsafe_path(
                &from,
                "only regular files and directories are allowed",
            ));
        }
    }
    Ok(())
}

fn reject_root_symlink(out: &Path) -> Result<(), CommandError> {
    match fs::symlink_metadata(out) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(unsafe_path(out, "documentation output cannot be a symlink"))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(read_error(out, source)),
    }
}

fn available_sibling(parent: &Path, label: &str) -> Result<PathBuf, CommandError> {
    for attempt in 0..128_u8 {
        let path = sibling(parent, label, attempt);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(path),
            Ok(_) => {}
            Err(source) => return Err(read_error(&path, source)),
        }
    }
    Err(write_error(
        parent,
        io::Error::new(io::ErrorKind::AlreadyExists, "no backup name was available"),
    ))
}

fn sibling(parent: &Path, label: &str, attempt: u8) -> PathBuf {
    parent.join(format!(
        ".recourse-{label}-{}-{attempt}",
        std::process::id()
    ))
}

fn unsafe_path(path: &Path, reason: &'static str) -> CommandError {
    CommandError::UnsafeDocumentation {
        path: path.to_owned(),
        reason,
    }
}

fn read_error(path: &Path, source: io::Error) -> CommandError {
    CommandError::Read {
        path: path.to_owned(),
        source,
    }
}

fn write_error(path: &Path, source: io::Error) -> CommandError {
    CommandError::Write {
        path: path.to_owned(),
        source,
    }
}
