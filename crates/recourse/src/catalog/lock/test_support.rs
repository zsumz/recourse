//! Unchecked state construction kept private to invariant-defense tests.

use super::{CatalogLock, LockEntry};

impl CatalogLock {
    pub(crate) fn set_type_base_unchecked(&mut self, type_base: String) {
        self.catalog.type_base = type_base;
    }

    pub(crate) fn replace_entries_unchecked(&mut self, entries: Vec<LockEntry>) {
        self.entries = entries;
    }
}
