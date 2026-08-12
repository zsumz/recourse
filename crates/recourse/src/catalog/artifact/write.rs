//! Capped canonical encoding before any caller-owned writer is mutated.

use std::io::{self, Write};

use serde::Serialize;

use super::{ArtifactWriteError, MAX_CATALOG_ARTIFACT_BYTES};

pub(super) fn pretty(value: &impl Serialize) -> Result<Vec<u8>, ArtifactWriteError> {
    let mut writer = CappedWriter::new();
    if let Err(error) = serde_json::to_writer_pretty(&mut writer, value) {
        return if writer.exceeded {
            Err(too_large())
        } else {
            Err(ArtifactWriteError::Serialize(error))
        };
    }
    writer.write_all(b"\n").map_err(|error| {
        if writer.exceeded {
            too_large()
        } else {
            ArtifactWriteError::Write(error)
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
        if self.body.len().saturating_add(buffer.len()) > MAX_CATALOG_ARTIFACT_BYTES {
            self.exceeded = true;
            return Err(io::Error::other("catalog artifact exceeds body limit"));
        }
        self.body.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

const fn too_large() -> ArtifactWriteError {
    ArtifactWriteError::TooLarge {
        maximum: MAX_CATALOG_ARTIFACT_BYTES,
    }
}
