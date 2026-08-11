//! Ownership-marked fallback transactions for documentation-tree replacement.

mod record;

use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use crate::error::CommandError;

use self::record::{
    backup_name, canonical_output, journal_body, journal_path, parent, reject_legacy_backup,
};
use super::{read_error, unsafe_path, write_error};

const OWNERSHIP_MARKER: &str = ".recourse-transaction-owner";
const MAX_JOURNAL_BYTES: usize = 4 * 1024;

pub(super) struct Transaction {
    journal: PathBuf,
    backup: PathBuf,
    body: Vec<u8>,
}

impl Transaction {
    pub(super) fn recover(out: &Path) -> Result<(), CommandError> {
        reject_legacy_backup(out)?;
        let journal = journal_path(out)?;
        let Some(body) = read_bounded(&journal)? else {
            return Ok(());
        };
        let transaction = Self::from_journal(out, journal, body)?;
        transaction.recover_state(out)
    }

    pub(super) fn begin(out: &Path) -> Result<Self, CommandError> {
        let journal = journal_path(out)?;
        let parent = parent(out);
        let canonical_output = canonical_output(out)?;
        for attempt in 0..128_u8 {
            let backup_name = format!(".recourse-backup-{}-{attempt}", std::process::id());
            let backup = parent.join(&backup_name);
            match fs::symlink_metadata(&backup) {
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(source) => return Err(read_error(&backup, source)),
            }
            let body = journal_body(&canonical_output, &backup_name)?;
            create_new(&journal, &body)?;
            return Ok(Self {
                journal,
                backup,
                body,
            });
        }
        Err(write_error(
            parent,
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "no unique documentation backup name was available",
            ),
        ))
    }

    pub(super) fn back_up(&self, out: &Path) -> Result<(), CommandError> {
        let marker = out.join(OWNERSHIP_MARKER);
        if let Err(error) = create_new(&marker, &self.body) {
            let _ = fs::remove_file(&self.journal);
            return Err(error);
        }
        if let Err(source) = fs::rename(out, &self.backup) {
            let _ = fs::remove_file(&marker);
            let _ = fs::remove_file(&self.journal);
            return Err(write_error(out, source));
        }
        Ok(())
    }

    pub(super) fn roll_back(&self, out: &Path) -> Result<(), io::Error> {
        fs::rename(&self.backup, out)?;
        fs::remove_file(out.join(OWNERSHIP_MARKER))?;
        fs::remove_file(&self.journal)
    }

    pub(super) fn finish(self, out: &Path) -> Result<(), CommandError> {
        self.validate(out)?;
        fs::remove_dir_all(&self.backup).map_err(|source| write_error(&self.backup, source))?;
        fs::remove_file(&self.journal).map_err(|source| write_error(&self.journal, source))
    }

    fn from_journal(out: &Path, journal: PathBuf, body: Vec<u8>) -> Result<Self, CommandError> {
        let backup_name = backup_name(out, &journal, &body)?;
        Ok(Self {
            journal,
            backup: parent(out).join(&backup_name),
            body,
        })
    }

    fn recover_state(self, out: &Path) -> Result<(), CommandError> {
        match fs::symlink_metadata(&self.backup) {
            Ok(metadata) => {
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(unsafe_path(
                        &self.backup,
                        "recovery backup is not a directory",
                    ));
                }
                self.validate(out)?;
                match fs::symlink_metadata(out) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        fs::rename(&self.backup, out).map_err(|source| write_error(out, source))?;
                        fs::remove_file(out.join(OWNERSHIP_MARKER))
                            .map_err(|source| write_error(out, source))?;
                        fs::remove_file(&self.journal)
                            .map_err(|source| write_error(&self.journal, source))
                    }
                    Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                        self.finish(out)
                    }
                    Ok(_) => Err(unsafe_path(
                        out,
                        "output conflicts with an interrupted documentation commit",
                    )),
                    Err(source) => Err(read_error(out, source)),
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                recover_before_backup(out, &self.journal, &self.body)
            }
            Err(source) => Err(read_error(&self.backup, source)),
        }
    }

    fn validate(&self, out: &Path) -> Result<(), CommandError> {
        let journal_body = read_bounded(&self.journal)?.ok_or_else(|| {
            unsafe_path(
                &self.journal,
                "documentation transaction journal is missing",
            )
        })?;
        if journal_body != self.body {
            return Err(unsafe_path(
                &self.journal,
                "documentation transaction journal changed",
            ));
        }
        let backup_name = backup_name(out, &self.journal, &journal_body)?;
        if parent(out).join(backup_name) != self.backup {
            return Err(unsafe_path(
                &self.journal,
                "documentation transaction backup changed",
            ));
        }
        let actual = fs::read(self.backup.join(OWNERSHIP_MARKER))
            .map_err(|source| read_error(&self.backup, source))?;
        if actual != self.body {
            return Err(unsafe_path(
                &self.backup,
                "recovery backup ownership marker does not match",
            ));
        }
        Ok(())
    }
}

fn recover_before_backup(out: &Path, journal: &Path, body: &[u8]) -> Result<(), CommandError> {
    let marker = out.join(OWNERSHIP_MARKER);
    match fs::read(&marker) {
        Ok(actual) if actual == body => {
            fs::remove_file(&marker).map_err(|source| write_error(&marker, source))?;
        }
        Ok(_) => {
            return Err(unsafe_path(
                &marker,
                "documentation ownership marker does not match",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => return Err(read_error(&marker, source)),
    }
    fs::remove_file(journal).map_err(|source| write_error(journal, source))
}

fn create_new(path: &Path, body: &[u8]) -> Result<(), CommandError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| write_error(path, source))?;
    if let Err(source) = file.write_all(body).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(write_error(path, source));
    }
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, CommandError> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(read_error(path, source)),
    };
    let mut body = Vec::new();
    file.take(MAX_JOURNAL_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|source| read_error(path, source))?;
    if body.len() > MAX_JOURNAL_BYTES {
        return Err(unsafe_path(
            path,
            "documentation transaction journal is too large",
        ));
    }
    Ok(Some(body))
}
