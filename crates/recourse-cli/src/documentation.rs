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
        let replacements = ReplacementIndex::new(lock);
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
                        replacements.get(diagnostic.code(), replacement.as_ref()),
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

#[derive(Clone, Copy)]
pub(super) struct ReplacementSummary<'a> {
    direct: &'a Code,
    terminal: &'a Code,
}

impl<'a> ReplacementSummary<'a> {
    pub(super) const fn direct(self) -> &'a Code {
        self.direct
    }

    pub(super) const fn terminal(self) -> &'a Code {
        self.terminal
    }
}

struct ReplacementIndex<'a> {
    entries: BTreeMap<&'a Code, &'a LockEntry>,
    summaries: BTreeMap<&'a Code, ReplacementSummary<'a>>,
}

impl<'a> ReplacementIndex<'a> {
    fn new(lock: &'a CatalogLock) -> Self {
        let entries = lock
            .entries()
            .iter()
            .map(|entry| (entry.code(), entry))
            .collect::<BTreeMap<_, _>>();
        let mut index = Self {
            entries,
            summaries: BTreeMap::new(),
        };
        for entry in lock.entries() {
            index.resolve(entry.code());
        }
        index
    }

    fn get(&self, code: &Code, direct: Option<&'a Code>) -> Option<ReplacementSummary<'a>> {
        direct.and_then(|_| self.summaries.get(code).copied())
    }

    fn resolve(&mut self, source: &'a Code) {
        if self.summaries.contains_key(source) {
            return;
        }
        let mut path = Vec::new();
        let mut current = source;
        let terminal = loop {
            if let Some(summary) = self.summaries.get(current) {
                break summary.terminal;
            }
            let Some(direct) = self
                .entries
                .get(current)
                .and_then(|entry| entry.replacement())
            else {
                break current;
            };
            path.push((current, direct));
            if path.len() > self.entries.len() {
                return;
            }
            current = direct;
        };
        for (code, direct) in path.into_iter().rev() {
            self.summaries
                .insert(code, ReplacementSummary { direct, terminal });
        }
    }
}

pub(crate) fn explain(
    diagnostic: &CatalogDiagnostic,
    artifact: &CatalogArtifact,
) -> Result<String, serde_json::Error> {
    page::active(diagnostic, artifact)
}
