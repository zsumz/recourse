//! Canonical RFC 3339 observation timestamps normalized to UTC.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

/// Validated canonical observation timestamp for a health finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationTime {
    instant: OffsetDateTime,
    wire: Box<str>,
}

impl ObservationTime {
    /// Normalizes an instant to a canonical UTC RFC 3339 representation.
    pub fn try_new(value: OffsetDateTime) -> Result<Self, ObservationTimeError> {
        let instant = value.to_offset(UtcOffset::UTC);
        let wire = instant
            .format(&Rfc3339)
            .map_err(ObservationTimeError::Format)?;
        Ok(Self {
            instant,
            wire: wire.into_boxed_str(),
        })
    }

    /// Parses RFC 3339 text and normalizes it to UTC.
    pub fn parse(value: &str) -> Result<Self, ObservationTimeError> {
        let instant =
            OffsetDateTime::parse(value, &Rfc3339).map_err(ObservationTimeError::Parse)?;
        Self::try_new(instant)
    }

    /// Canonical UTC RFC 3339 wire text.
    pub fn as_str(&self) -> &str {
        &self.wire
    }

    /// Normalized UTC instant.
    pub const fn instant(&self) -> OffsetDateTime {
        self.instant
    }
}

impl Display for ObservationTime {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for ObservationTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ObservationTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = <Box<str>>::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

/// Invalid or unformattable observation time.
#[derive(Debug)]
pub enum ObservationTimeError {
    /// Input was not RFC 3339 timestamp text.
    Parse(time::error::Parse),
    /// Instant cannot be represented by the RFC 3339 profile.
    Format(time::error::Format),
}

impl Display for ObservationTimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "parse observation time: {error}"),
            Self::Format(error) => write!(formatter, "format observation time: {error}"),
        }
    }
}

impl Error for ObservationTimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::Format(error) => Some(error),
        }
    }
}
