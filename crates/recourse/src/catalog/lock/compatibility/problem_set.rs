//! Governed API-operation Problem-set compatibility.

use crate::catalog::{CatalogArtifact, Code};

use super::{CatalogLock, CompatibilityChange, change::ChangeInput, push};

pub(super) fn compare(
    lock: &CatalogLock,
    current: &CatalogArtifact,
    changes: &mut Vec<CompatibilityChange>,
) {
    for (id, previous) in lock.problem_sets() {
        let Some(current) = current.problem_sets().get(id) else {
            push(changes, ChangeInput::problem_set_removed(id));
            continue;
        };
        compare_members(id, previous, current, changes);
    }
    for id in current.problem_sets().keys() {
        if !lock.problem_sets().contains_key(id) {
            push(changes, ChangeInput::problem_set_added(id));
        }
    }
}

fn compare_members(
    id: &str,
    previous: &[Code],
    current: &[Code],
    changes: &mut Vec<CompatibilityChange>,
) {
    for code in previous {
        if !current.contains(code) {
            push(changes, ChangeInput::problem_set_member_removed(id, code));
        }
    }
    for code in current {
        if !previous.contains(code) {
            push(changes, ChangeInput::problem_set_member_added(id, code));
        }
    }
}
