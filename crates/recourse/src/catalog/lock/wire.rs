//! Private deserialization shapes converted into validated lock domain values.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::catalog::{Code, CodeNumber, artifact::CatalogDiagnosticWire};

use super::{CURRENT_SCHEMA_VERSION, CatalogLock, LockEntry, LockIdentity};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogLockWire {
    schema_version: u32,
    catalog: LockIdentity,
    entries: Vec<LockEntryWire>,
    #[serde(default)]
    problem_sets: OptionalProblemSets,
}

impl CatalogLockWire {
    pub(super) const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub(super) const fn has_problem_sets(&self) -> bool {
        self.problem_sets.0.is_some()
    }

    pub(super) fn into_domain(self) -> CatalogLock {
        CatalogLock {
            schema_version: CURRENT_SCHEMA_VERSION,
            catalog: self.catalog,
            entries: self
                .entries
                .into_iter()
                .map(LockEntryWire::into_domain)
                .collect(),
            problem_sets: self.problem_sets.0.unwrap_or_default(),
        }
    }
}

#[derive(Default)]
struct OptionalProblemSets(Option<BTreeMap<String, Vec<Code>>>);

impl<'de> Deserialize<'de> for OptionalProblemSets {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        BTreeMap::deserialize(deserializer).map(|value| Self(Some(value)))
    }
}

#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum LockEntryWire {
    Reserved {
        number: CodeNumber,
        code: Code,
        #[serde(rename = "type")]
        type_uri: String,
    },
    Active {
        diagnostic: CatalogDiagnosticWire,
    },
    Retired {
        diagnostic: CatalogDiagnosticWire,
        reason: String,
        replacement: Option<Code>,
    },
}

impl LockEntryWire {
    fn into_domain(self) -> LockEntry {
        match self {
            Self::Reserved {
                number,
                code,
                type_uri,
            } => LockEntry::Reserved {
                number,
                code,
                type_uri,
            },
            Self::Active { diagnostic } => LockEntry::Active {
                diagnostic: diagnostic.into_domain(),
            },
            Self::Retired {
                diagnostic,
                reason,
                replacement,
            } => LockEntry::Retired {
                diagnostic: diagnostic.into_domain(),
                reason,
                replacement,
            },
        }
    }
}
