//! Private deserialization shapes converted into validated lock domain values.

use serde::Deserialize;

use crate::catalog::{Code, CodeNumber, artifact::CatalogDiagnosticWire};

use super::{CatalogLock, LockEntry, LockIdentity};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogLockWire {
    schema_version: u32,
    catalog: LockIdentity,
    entries: Vec<LockEntryWire>,
}

impl CatalogLockWire {
    pub(super) fn into_domain(self) -> CatalogLock {
        CatalogLock {
            schema_version: self.schema_version,
            catalog: self.catalog,
            entries: self
                .entries
                .into_iter()
                .map(LockEntryWire::into_domain)
                .collect(),
        }
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
