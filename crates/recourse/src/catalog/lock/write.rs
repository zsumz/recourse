//! Capped lock encoding before any caller-owned writer is mutated.

use std::io::{self, Write};

use serde::Serialize;

use super::{CatalogLock, LockWriteError, MAX_CATALOG_LOCK_BYTES};

pub(super) fn write_pretty<W: Write>(
    lock: &CatalogLock,
    mut writer: W,
) -> Result<(), LockWriteError> {
    let body = pretty(lock)?;
    writer.write_all(&body).map_err(LockWriteError::Write)
}

pub(super) fn pretty(value: &impl Serialize) -> Result<Vec<u8>, LockWriteError> {
    let mut writer = CappedWriter::new();
    if let Err(error) = serde_json::to_writer_pretty(&mut writer, value) {
        return if writer.exceeded {
            Err(too_large())
        } else {
            Err(LockWriteError::Serialize(error))
        };
    }
    writer.write_all(b"\n").map_err(|error| {
        if writer.exceeded {
            too_large()
        } else {
            LockWriteError::Write(error)
        }
    })?;
    Ok(writer.body)
}

struct CappedWriter {
    body: Vec<u8>,
    exceeded: bool,
}

impl CappedWriter {
    fn new() -> Self {
        Self {
            body: Vec::new(),
            exceeded: false,
        }
    }
}

impl Write for CappedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.body.len().saturating_add(buffer.len()) > MAX_CATALOG_LOCK_BYTES {
            self.exceeded = true;
            return Err(io::Error::other("catalog lock exceeds body limit"));
        }
        self.body.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

const fn too_large() -> LockWriteError {
    LockWriteError::TooLarge {
        maximum: MAX_CATALOG_LOCK_BYTES,
    }
}
