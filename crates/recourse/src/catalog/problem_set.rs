//! Stable API-operation declarations over governed HTTP diagnostics.

use std::{any::TypeId, fmt, marker::PhantomData};

use crate::http::HttpProblemType;

use super::{CatalogSpec, CodeNumber};

/// Maximum UTF-8 bytes in a stable problem-set operation ID.
pub const MAX_PROBLEM_SET_ID_BYTES: usize = 128;

pub(crate) fn valid_problem_set_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_PROBLEM_SET_ID_BYTES
        && id
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Declared HTTP diagnostics one stable API operation may emit.
pub struct ProblemSet<C: CatalogSpec> {
    pub(crate) id: String,
    pub(crate) members: Vec<ProblemSetMember>,
    marker: PhantomData<fn() -> C>,
}

impl<C: CatalogSpec> ProblemSet<C> {
    /// Starts a declaration for one stable API operation ID.
    pub fn builder(id: impl Into<String>) -> ProblemSetBuilder<C> {
        ProblemSetBuilder {
            id: id.into(),
            members: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<C: CatalogSpec> fmt::Debug for ProblemSet<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProblemSet")
            .field("id", &self.id)
            .field("members", &self.members)
            .finish()
    }
}

/// Consuming builder for one operation's explicit diagnostic set.
pub struct ProblemSetBuilder<C: CatalogSpec> {
    id: String,
    members: Vec<ProblemSetMember>,
    marker: PhantomData<fn() -> C>,
}

impl<C: CatalogSpec> ProblemSetBuilder<C> {
    /// Includes one HTTP diagnostic marker in the operation declaration.
    #[must_use]
    pub fn include<D>(mut self) -> Self
    where
        D: HttpProblemType<Catalog = C>,
    {
        self.members.push(ProblemSetMember {
            type_id: TypeId::of::<D>(),
            number: D::NUMBER,
        });
        self
    }

    /// Finishes the declaration for validation with its catalog.
    pub fn build(self) -> ProblemSet<C> {
        ProblemSet {
            id: self.id,
            members: self.members,
            marker: PhantomData,
        }
    }
}

impl<C: CatalogSpec> fmt::Debug for ProblemSetBuilder<C> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProblemSetBuilder")
            .field("id", &self.id)
            .field("members", &self.members)
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct ProblemSetMember {
    pub(crate) type_id: TypeId,
    pub(crate) number: CodeNumber,
}
