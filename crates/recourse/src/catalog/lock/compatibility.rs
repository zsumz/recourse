//! Conservative catalog compatibility classification against accepted history.

mod change;
mod schema;

use crate::catalog::{CatalogArtifact, CatalogDiagnostic};

use super::{CatalogLock, LockEntry};
use change::{ChangeInput, push};
pub use change::{CompatibilityChange, CompatibilityReport, CompatibilitySeverity};

pub(super) fn check(lock: &CatalogLock, current: &CatalogArtifact) -> CompatibilityReport {
    let mut changes = Vec::new();
    compare_namespace(lock, current, &mut changes);
    if changes
        .iter()
        .any(|change| change.severity() == CompatibilitySeverity::Forbidden)
    {
        return CompatibilityReport::new(changes);
    }
    for entry in &lock.entries {
        compare_entry(entry, current, &mut changes);
    }
    for diagnostic in current.diagnostics() {
        if !lock
            .entries
            .iter()
            .any(|entry| entry.number() == diagnostic.number())
        {
            push(
                &mut changes,
                ChangeInput::diagnostic_added(diagnostic.code()),
            );
        }
    }
    CompatibilityReport::new(changes)
}

fn compare_namespace(
    lock: &CatalogLock,
    current: &CatalogArtifact,
    changes: &mut Vec<CompatibilityChange>,
) {
    for (path, previous, value) in [
        ("catalog.name", lock.name(), current.name()),
        ("catalog.prefix", lock.prefix(), current.prefix()),
        ("catalog.type_base", lock.type_base(), current.type_base()),
    ] {
        if previous != value {
            push(changes, ChangeInput::namespace(path, previous, value));
        }
    }
}

fn compare_entry(
    entry: &LockEntry,
    current: &CatalogArtifact,
    changes: &mut Vec<CompatibilityChange>,
) {
    let diagnostic = current
        .diagnostics()
        .iter()
        .find(|value| value.number() == entry.number());
    match (entry, diagnostic) {
        (LockEntry::Reserved { .. }, Some(value)) => {
            push(changes, ChangeInput::reservation_activated(value.code()));
        }
        (
            LockEntry::Active {
                diagnostic: previous,
            },
            Some(value),
        ) => {
            compare_diagnostic(previous, value, changes);
        }
        (LockEntry::Active { diagnostic }, None) => {
            push(changes, ChangeInput::active_missing(diagnostic.code()));
        }
        (LockEntry::Retired { diagnostic, .. }, Some(_)) => {
            push(changes, ChangeInput::retired_reused(diagnostic.code()));
        }
        (LockEntry::Reserved { .. } | LockEntry::Retired { .. }, None) => {}
    }
}

fn compare_diagnostic(
    previous: &CatalogDiagnostic,
    current: &CatalogDiagnostic,
    changes: &mut Vec<CompatibilityChange>,
) {
    let code = previous.code();
    if previous.title() != current.title() {
        push(
            changes,
            ChangeInput::title(code, previous.title(), current.title()),
        );
    }
    if previous.detail() != current.detail()
        || previous.suggestions() != current.suggestions()
        || previous.documentation_markdown() != current.documentation_markdown()
    {
        push(changes, ChangeInput::metadata_improved(code));
    }
    schema::compare(
        code,
        "evidence_schema",
        previous.evidence_schema(),
        current.evidence_schema(),
        changes,
    );
    compare_surfaces(previous, current, changes);
}

fn compare_surfaces(
    previous: &CatalogDiagnostic,
    current: &CatalogDiagnostic,
    changes: &mut Vec<CompatibilityChange>,
) {
    compare_http(previous, current, changes);
    compare_operation(previous, current, changes);
    compare_health(previous, current, changes);
}

fn compare_http(
    previous: &CatalogDiagnostic,
    current: &CatalogDiagnostic,
    changes: &mut Vec<CompatibilityChange>,
) {
    let code = previous.code();
    match (previous.http_status(), current.http_status()) {
        (None, Some(_)) => push(changes, ChangeInput::surface_added(code, "http")),
        (Some(_), None) => push(changes, ChangeInput::surface_removed(code, "http")),
        (Some(old), Some(new)) => {
            if old != new {
                push(changes, ChangeInput::http_status(code, old, new));
            }
            if previous.http_policy() != current.http_policy()
                || previous.required_headers() != current.required_headers()
            {
                push(changes, ChangeInput::http_contract(code));
            }
        }
        (None, None) => {}
    }
}

fn compare_operation(
    previous: &CatalogDiagnostic,
    current: &CatalogDiagnostic,
    changes: &mut Vec<CompatibilityChange>,
) {
    let code = previous.code();
    match (previous.impact_schema(), current.impact_schema()) {
        (None, Some(_)) => push(changes, ChangeInput::surface_added(code, "operation")),
        (Some(_), None) => push(changes, ChangeInput::surface_removed(code, "operation")),
        (Some(old), Some(new)) => {
            schema::compare(code, "surfaces.operation.impact_schema", old, new, changes);
        }
        (None, None) => {}
    }
}

fn compare_health(
    previous: &CatalogDiagnostic,
    current: &CatalogDiagnostic,
    changes: &mut Vec<CompatibilityChange>,
) {
    let code = previous.code();
    match (previous.supports_health(), current.supports_health()) {
        (false, true) => push(changes, ChangeInput::surface_added(code, "health")),
        (true, false) => push(changes, ChangeInput::surface_removed(code, "health")),
        (false, false) | (true, true) => {}
    }
}
