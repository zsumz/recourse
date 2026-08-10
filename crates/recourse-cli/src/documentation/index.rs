//! Catalog-wide active, retired, and reserved documentation index.

use std::fmt::Write as _;

use recourse::catalog::{CatalogArtifact, CatalogLock, LockEntry};

use super::markdown;

pub(super) fn render(artifact: &CatalogArtifact, lock: &CatalogLock) -> String {
    let mut body = format!("# {} diagnostics\n\n", markdown::text(artifact.name()));
    body.push_str("Generated from the accepted Recourse catalog and lock.\n\n## Active\n\n");
    for diagnostic in artifact.diagnostics() {
        let _ = writeln!(
            body,
            "- [{}]({}.md): {}",
            diagnostic.code(),
            diagnostic.code(),
            markdown::text(diagnostic.title())
        );
    }
    push_retired(&mut body, lock);
    push_reserved(&mut body, lock);
    body
}

fn push_retired(body: &mut String, lock: &CatalogLock) {
    let retired = lock
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            LockEntry::Retired { diagnostic, .. } => Some(diagnostic),
            LockEntry::Reserved { .. } | LockEntry::Active { .. } => None,
        })
        .collect::<Vec<_>>();
    if retired.is_empty() {
        return;
    }
    body.push_str("\n## Retired\n\n");
    for diagnostic in retired {
        let _ = writeln!(
            body,
            "- [{}](retired/{}.md): {}",
            diagnostic.code(),
            diagnostic.code(),
            markdown::text(diagnostic.title())
        );
    }
}

fn push_reserved(body: &mut String, lock: &CatalogLock) {
    let reserved = lock
        .entries()
        .iter()
        .filter(|entry| matches!(entry, LockEntry::Reserved { .. }))
        .collect::<Vec<_>>();
    if reserved.is_empty() {
        return;
    }
    body.push_str("\n## Reserved\n\n");
    for entry in reserved {
        let _ = writeln!(body, "- `{}` — `{}`", entry.code(), entry.type_uri());
    }
}
