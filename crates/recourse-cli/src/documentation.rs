//! Deterministic generated documentation assembled from governed artifacts.

mod index;
mod markdown;
mod page;
mod schema;

use std::{collections::BTreeMap, path::PathBuf};

use recourse::catalog::{CatalogArtifact, CatalogDiagnostic, CatalogLock, LockEntry};

#[derive(Debug)]
pub(crate) struct GeneratedDocumentation {
    pages: BTreeMap<PathBuf, String>,
}

impl GeneratedDocumentation {
    pub(crate) fn render(
        artifact: &CatalogArtifact,
        lock: &CatalogLock,
    ) -> Result<Self, serde_json::Error> {
        let mut pages = BTreeMap::new();
        pages.insert(PathBuf::from("index.md"), index::render(artifact, lock));
        for diagnostic in artifact.diagnostics() {
            pages.insert(
                PathBuf::from(format!("{}.md", diagnostic.code())),
                page::active(diagnostic, artifact)?,
            );
        }
        for entry in lock.entries() {
            if let LockEntry::Retired {
                diagnostic,
                reason,
                replacement,
            } = entry
            {
                pages.insert(
                    PathBuf::from(format!("retired/{}.md", diagnostic.code())),
                    page::retired(diagnostic, reason, replacement.as_ref())?,
                );
            }
        }
        Ok(Self { pages })
    }

    pub(crate) fn pages(&self) -> &BTreeMap<PathBuf, String> {
        &self.pages
    }
}

pub(crate) fn explain(
    diagnostic: &CatalogDiagnostic,
    artifact: &CatalogArtifact,
) -> Result<String, serde_json::Error> {
    page::active(diagnostic, artifact)
}
