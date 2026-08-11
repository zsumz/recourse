//! Symlink-safe copying, staging cleanup, and output-tree commit.

use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use crate::error::CommandError;

#[cfg(test)]
#[path = "tests/recovery.rs"]
mod recovery_test;

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
        recover_interrupted_commit(out)?;
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
        if exchange_directories(&self.path, out).is_ok() {
            let _ = fs::remove_dir_all(&self.path);
            return Ok(());
        }
        let backup = backup_path(out)?;
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
        let _ = fs::remove_dir_all(&backup);
        Ok(())
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

fn recover_interrupted_commit(out: &Path) -> Result<(), CommandError> {
    let backup = backup_path(out)?;
    let backup_metadata = match fs::symlink_metadata(&backup) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(read_error(&backup, source)),
    };
    if !backup_metadata.is_dir() || backup_metadata.file_type().is_symlink() {
        return Err(unsafe_path(&backup, "recovery backup is not a directory"));
    }
    match fs::symlink_metadata(out) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::rename(&backup, out).map_err(|source| write_error(out, source))
        }
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(&backup).map_err(|source| write_error(&backup, source))
        }
        Ok(_) => Err(unsafe_path(
            out,
            "output conflicts with an interrupted documentation commit",
        )),
        Err(source) => Err(read_error(out, source)),
    }
}

fn backup_path(out: &Path) -> Result<PathBuf, CommandError> {
    let name = out
        .file_name()
        .ok_or_else(|| unsafe_path(out, "output must name a directory"))?;
    let mut backup_name = OsString::from(".recourse-backup-");
    backup_name.push(name);
    Ok(out.with_file_name(backup_name))
}

fn sibling(parent: &Path, label: &str, attempt: u8) -> PathBuf {
    parent.join(format!(
        ".recourse-{label}-{}-{attempt}",
        std::process::id()
    ))
}

#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
fn exchange_directories(staged: &Path, live: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};

    renameat_with(CWD, staged, CWD, live, RenameFlags::EXCHANGE).map_err(io::Error::from)
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
fn exchange_directories(_staged: &Path, _live: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic directory exchange is unavailable",
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
