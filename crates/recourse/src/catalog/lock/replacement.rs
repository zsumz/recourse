//! Replacement-target and acyclic-chain invariants.

use std::collections::HashMap;

use crate::catalog::Code;

use super::{CatalogLock, LockState};

#[derive(Debug)]
pub(super) enum ReplacementIssue {
    MissingOrReserved { source: Code, replacement: Code },
    Cycle { code: Code },
}

pub(super) fn validate(lock: &CatalogLock) -> Result<(), ReplacementIssue> {
    let indices = lock
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.code(), index))
        .collect::<HashMap<_, _>>();
    validate_targets(lock, &indices)?;
    validate_acyclic(lock, &indices)
}

fn validate_targets(
    lock: &CatalogLock,
    indices: &HashMap<&Code, usize>,
) -> Result<(), ReplacementIssue> {
    for entry in &lock.entries {
        let Some(replacement) = entry.replacement() else {
            continue;
        };
        let valid = indices
            .get(replacement)
            .is_some_and(|index| lock.entries[*index].state() != LockState::Reserved);
        if !valid {
            return Err(ReplacementIssue::MissingOrReserved {
                source: entry.code().clone(),
                replacement: replacement.clone(),
            });
        }
    }
    Ok(())
}

fn validate_acyclic(
    lock: &CatalogLock,
    indices: &HashMap<&Code, usize>,
) -> Result<(), ReplacementIssue> {
    let mut state = vec![0_u8; lock.entries.len()];
    for start in 0..lock.entries.len() {
        if state[start] != 0 {
            continue;
        }
        let mut path = Vec::new();
        let mut cursor = Some(start);
        while let Some(index) = cursor {
            match state[index] {
                1 => {
                    return Err(ReplacementIssue::Cycle {
                        code: lock.entries[index].code().clone(),
                    });
                }
                2 => break,
                _ => {}
            }
            state[index] = 1;
            path.push(index);
            cursor = lock.entries[index]
                .replacement()
                .and_then(|replacement| indices.get(replacement).copied());
        }
        for index in path {
            state[index] = 2;
        }
    }
    Ok(())
}
