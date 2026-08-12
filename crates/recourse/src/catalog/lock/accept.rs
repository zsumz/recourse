//! Compatibility-gated acceptance and explicit retirement transitions.

use crate::catalog::{CatalogArtifact, Code};

use super::{
    AcceptanceError, CatalogLock, CompatibilityReport, LockEntry, LockState, RetirementError,
    closure, compatibility,
    replacement::{self, ReplacementIssue},
    retirement,
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
    let mut candidate = lock.clone();
    for diagnostic in current.diagnostics() {
        let number = diagnostic.number();
        match candidate
            .entries
            .binary_search_by_key(&number, LockEntry::number)
        {
            Ok(index) => candidate.entries[index] = LockEntry::active(diagnostic.clone()),
            Err(index) => candidate
                .entries
                .insert(index, LockEntry::active(diagnostic.clone())),
        }
    }
    candidate.problem_sets = current.problem_sets().clone();
    closure::validate(&candidate)
        .map_err(|reason| AcceptanceError::InvalidGeneratedLock { reason })?;
    *lock = candidate;
    Ok(report)
}

pub(super) fn retire<'a>(
    lock: &'a mut CatalogLock,
    code: &Code,
    reason: String,
    replacement: Option<Code>,
) -> Result<&'a LockEntry, RetirementError> {
    retirement::validate(&reason).map_err(RetirementError::from)?;
    let mut candidate = lock.clone();
    let index = candidate
        .entries
        .iter()
        .position(|entry| entry.code() == code)
        .ok_or_else(|| RetirementError::UnknownCode { code: code.clone() })?;
    if replacement.as_ref() == Some(code) {
        return Err(RetirementError::InvalidReplacement { code: code.clone() });
    }
    let state = candidate.entries[index].state();
    let Some(diagnostic) = candidate.entries[index].diagnostic().cloned() else {
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
    let retired = LockEntry::Retired {
        diagnostic,
        reason,
        replacement,
    };
    candidate.entries[index] = retired;
    for members in candidate.problem_sets.values_mut() {
        members.retain(|member| member != code);
    }
    if let Err(issue) = replacement::validate(&candidate) {
        return Err(retirement_error(issue));
    }
    closure::validate(&candidate)
        .map_err(|reason| RetirementError::InvalidGeneratedLock { reason })?;
    *lock = candidate;
    Ok(&lock.entries[index])
}

fn retirement_error(issue: ReplacementIssue) -> RetirementError {
    match issue {
        ReplacementIssue::MissingOrReserved { replacement, .. } => {
            RetirementError::InvalidReplacement { code: replacement }
        }
        ReplacementIssue::Cycle { code } => RetirementError::ReplacementCycle { code },
    }
}
