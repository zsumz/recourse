//! Compatibility-gated acceptance and explicit retirement transitions.

use crate::catalog::{CatalogArtifact, Code};

use super::{
    AcceptanceError, CatalogLock, CompatibilityReport, LockEntry, LockState, RetirementError,
    compatibility,
};

/// Whether catalog acceptance may record acknowledged breaking changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptanceMode {
    /// Accept only compatible differences.
    CompatibleOnly,
    /// Accept breaking differences while still refusing forbidden history changes.
    AcknowledgeBreaking,
}

pub(super) fn accept(
    lock: &mut CatalogLock,
    current: &CatalogArtifact,
    mode: AcceptanceMode,
) -> Result<CompatibilityReport, AcceptanceError> {
    let report = compatibility::check(lock, current);
    if report.has_forbidden() {
        return Err(AcceptanceError::Forbidden(report));
    }
    if report.has_breaking() && mode == AcceptanceMode::CompatibleOnly {
        return Err(AcceptanceError::BreakingRequiresAcknowledgement(report));
    }
    for diagnostic in current.diagnostics() {
        let number = diagnostic.number();
        match lock
            .entries
            .binary_search_by_key(&number, LockEntry::number)
        {
            Ok(index) => lock.entries[index] = LockEntry::active(diagnostic.clone()),
            Err(index) => lock
                .entries
                .insert(index, LockEntry::active(diagnostic.clone())),
        }
    }
    Ok(report)
}

pub(super) fn retire<'a>(
    lock: &'a mut CatalogLock,
    code: &Code,
    reason: String,
    replacement: Option<Code>,
) -> Result<&'a LockEntry, RetirementError> {
    if reason.trim().is_empty() {
        return Err(RetirementError::EmptyReason);
    }
    let index = lock
        .entries
        .iter()
        .position(|entry| entry.code() == code)
        .ok_or_else(|| RetirementError::UnknownCode { code: code.clone() })?;
    validate_replacement(lock, code, replacement.as_ref())?;
    let state = lock.entries[index].state();
    let Some(diagnostic) = lock.entries[index].diagnostic().cloned() else {
        return Err(RetirementError::NotActive {
            code: code.clone(),
            state,
        });
    };
    if state != LockState::Active {
        return Err(RetirementError::NotActive {
            code: code.clone(),
            state,
        });
    }
    lock.entries[index] = LockEntry::Retired {
        diagnostic,
        reason,
        replacement,
    };
    Ok(&lock.entries[index])
}

fn validate_replacement(
    lock: &CatalogLock,
    retiring: &Code,
    replacement: Option<&Code>,
) -> Result<(), RetirementError> {
    let Some(replacement) = replacement else {
        return Ok(());
    };
    let active = lock
        .entries
        .iter()
        .any(|entry| entry.code() == replacement && entry.state() == LockState::Active);
    if replacement == retiring || !active {
        return Err(RetirementError::InvalidReplacement {
            code: replacement.clone(),
        });
    }
    Ok(())
}
