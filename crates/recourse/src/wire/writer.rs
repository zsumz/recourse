//! Capped canonical JSON output.

use std::io::{self, Write};

use serde::Serialize;

use super::{WireLimit, WireLimitError, WireLimits};

#[derive(Debug)]
pub(crate) enum BoundedJsonError {
    Serialize(serde_json::Error),
    Limit(WireLimitError),
}

pub(crate) fn to_bounded_vec<T: Serialize>(
    value: &T,
    limits: WireLimits,
) -> Result<Vec<u8>, BoundedJsonError> {
    let mut writer = CappedWriter::new(limits.max_body_bytes());
    if let Err(error) = serde_json::to_writer(&mut writer, value) {
        return match writer.exceeded {
            Some(limit) => Err(BoundedJsonError::Limit(limit)),
            None => Err(BoundedJsonError::Serialize(error)),
        };
    }
    Ok(writer.body)
}

struct CappedWriter {
    body: Vec<u8>,
    maximum: usize,
    exceeded: Option<WireLimitError>,
}

impl CappedWriter {
    fn new(maximum: usize) -> Self {
        Self {
            body: Vec::new(),
            maximum,
            exceeded: None,
        }
    }
}

impl Write for CappedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let actual = self.body.len().saturating_add(buffer.len());
        if actual > self.maximum {
            self.exceeded = Some(WireLimitError::new(
                WireLimit::BodyBytes,
                self.maximum,
                actual,
            ));
            return Err(io::Error::other("diagnostic body exceeds wire limit"));
        }
        self.body.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
