//! Deterministic generated documentation assembled from governed artifacts.

mod index;
mod markdown;
mod page;
mod schema;

use std::{collections::BTreeMap, path::PathBuf};

use recourse::catalog::{CatalogArtifact, CatalogDiagnostic, CatalogLock, Code, LockEntry};

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
                    page::retired(
                        diagnostic,
                        reason,
                        &replacement_chain(lock, replacement.as_ref()),
                    )?,
                );
            }
        }
        Ok(Self { pages })
    }

    pub(crate) fn pages(&self) -> &BTreeMap<PathBuf, String> {
        &self.pages
    }
}

fn replacement_chain<'a>(lock: &'a CatalogLock, first: Option<&'a Code>) -> Vec<&'a Code> {
    let mut chain = Vec::new();
    let mut current = first;
    for _ in 0..lock.entries().len() {
        let Some(code) = current else {
            break;
        };
        chain.push(code);
        current = lock
            .entries()
            .iter()
            .find(|entry| entry.code() == code)
            .and_then(LockEntry::replacement);
    }
    chain
}

pub(crate) fn explain(
    diagnostic: &CatalogDiagnostic,
    artifact: &CatalogArtifact,
) -> Result<String, serde_json::Error> {
    page::active(diagnostic, artifact)
}
