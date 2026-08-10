//! Deterministic lock writing and never-reusing reservation allocation.

use std::io::Write;

use crate::catalog::{Code, CodeNumber};

use super::{CatalogLock, LockEntry, LockWriteError, ReservationError};

/// Requested reservation allocation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reservation {
    /// Allocate one number larger than every historical entry.
    Next,
    /// Allocate this exact never-before-used positive number.
    Exact(CodeNumber),
}

pub(super) fn reserve(
    lock: &mut CatalogLock,
    reservation: Reservation,
) -> Result<&LockEntry, ReservationError> {
    let number = match reservation {
        Reservation::Next => next_number(lock)?,
        Reservation::Exact(number) => {
            if lock.entries.iter().any(|entry| entry.number() == number) {
                return Err(ReservationError::AlreadyUsed { number });
            }
            number
        }
    };
    let code = Code::new(lock.prefix(), number).map_err(|_| ReservationError::InvalidLockPrefix)?;
    let type_uri = format!("{}{code}", lock.type_base());
    let index = lock
        .entries
        .binary_search_by_key(&number, LockEntry::number)
        .unwrap_or_else(|index| index);
    lock.entries
        .insert(index, LockEntry::reserved(number, code, type_uri));
    Ok(&lock.entries[index])
}

fn next_number(lock: &CatalogLock) -> Result<CodeNumber, ReservationError> {
    let next = match lock.entries.last() {
        Some(entry) => entry
            .number()
            .get()
            .checked_add(1)
            .ok_or(ReservationError::NumberSpaceExhausted)?,
        None => 1,
    };
    CodeNumber::try_new(next).map_err(|_| ReservationError::NumberSpaceExhausted)
}

pub(super) fn write_pretty<W: Write>(
    lock: &CatalogLock,
    mut writer: W,
) -> Result<(), LockWriteError> {
    serde_json::to_writer_pretty(&mut writer, lock).map_err(LockWriteError::Serialize)?;
    writer.write_all(b"\n").map_err(LockWriteError::Write)
}
