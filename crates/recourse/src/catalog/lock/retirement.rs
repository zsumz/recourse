//! Shared retirement-reason constraints for mutation and lock parsing.

use std::fmt::{self, Display, Formatter};

/// Maximum Unicode scalar values accepted in a retirement rationale.
pub const MAX_RETIREMENT_REASON_CHARS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReasonViolation {
    Empty,
    TooLong { actual_chars: usize, maximum: usize },
    ControlCharacter { character_index: usize },
}

pub(crate) fn validate(reason: &str) -> Result<(), ReasonViolation> {
    if reason.trim().is_empty() {
        return Err(ReasonViolation::Empty);
    }
    let actual_chars = reason.chars().count();
    if actual_chars > MAX_RETIREMENT_REASON_CHARS {
        return Err(ReasonViolation::TooLong {
            actual_chars,
            maximum: MAX_RETIREMENT_REASON_CHARS,
        });
    }
    if let Some((character_index, _)) = reason
        .chars()
        .enumerate()
        .find(|(_, character)| character.is_control())
    {
        return Err(ReasonViolation::ControlCharacter { character_index });
    }
    Ok(())
}

impl Display for ReasonViolation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("retirement reason must be nonempty"),
            Self::TooLong {
                actual_chars,
                maximum,
            } => write!(
                formatter,
                "retirement reason is {actual_chars} characters; maximum is {maximum}"
            ),
            Self::ControlCharacter { character_index } => write!(
                formatter,
                "retirement reason contains a control character at index {character_index}"
            ),
        }
    }
}
