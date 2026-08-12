//! Deterministic lock writing and never-reusing reservation allocation.

use crate::{
    catalog::{Code, CodeNumber, maximum_type_uri_bytes, type_namespace_fits_wire},
    wire::WireLimits,
};

use super::{CatalogLock, LockEntry, ReservationError, closure};

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
    if !type_namespace_fits_wire(lock.type_base(), lock.prefix()) {
        return Err(ReservationError::TypeNamespaceTooLong {
            maximum: WireLimits::DEFAULT_MAX_STRING_BYTES,
            actual: maximum_type_uri_bytes(lock.type_base(), lock.prefix()),
        });
    }
    let mut candidate = lock.clone();
    let number = match reservation {
        Reservation::Next => next_number(&candidate)?,
        Reservation::Exact(number) => {
            if candidate
                .entries
                .iter()
                .any(|entry| entry.number() == number)
            {
                return Err(ReservationError::AlreadyUsed { number });
            }
            number
        }
    };
    let code =
        Code::new(candidate.prefix(), number).map_err(|_| ReservationError::InvalidLockPrefix)?;
    let type_uri = format!("{}{code}", candidate.type_base());
    let index = candidate
        .entries
        .binary_search_by_key(&number, LockEntry::number)
        .unwrap_or_else(|index| index);
    candidate
        .entries
        .insert(index, LockEntry::reserved(number, code, type_uri));
    closure::validate(&candidate)
        .map_err(|reason| ReservationError::InvalidGeneratedLock { reason })?;
    *lock = candidate;
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
