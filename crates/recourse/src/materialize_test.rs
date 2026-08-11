//! Streaming caps, object roots, and duplicate-member materialization tests.

use std::cell::Cell;

use serde::{Serialize, Serializer, ser::SerializeMap};

use super::{
    materialize::{MaterializeError, object},
    wire::{WireLimit, WireLimits},
};

struct DuplicateMembers;

impl Serialize for DuplicateMembers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("same", &1)?;
        map.serialize_entry("same", &2)?;
        map.end()
    }
}

struct StreamingItems<'a>(&'a Cell<usize>);

impl Serialize for StreamingItems<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let values = (0..1_000_000).inspect(|_| self.0.set(self.0.get() + 1));
        serializer.collect_seq(values)
    }
}

struct StreamingObject<'a>(&'a Cell<usize>);

impl Serialize for StreamingObject<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("items", &StreamingItems(self.0))?;
        map.end()
    }
}

#[test]
fn duplicate_members_are_rejected_instead_of_collapsed() {
    let error = object(&DuplicateMembers, WireLimits::default())
        .err()
        .unwrap_or_else(|| panic!("duplicate members must fail"));
    assert!(matches!(error, MaterializeError::Json(_)));
}

#[test]
fn streaming_serialization_stops_at_the_body_cap() {
    let visited = Cell::new(0);
    let error = object(&StreamingObject(&visited), WireLimits::default())
        .err()
        .unwrap_or_else(|| panic!("oversized stream must fail"));
    assert!(matches!(
        error,
        MaterializeError::Limit(limit) if limit.limit() == WireLimit::BodyBytes
    ));
    assert!(visited.get() < 1_000_000);
}

#[test]
fn scalar_roots_remain_distinct_from_serialization_failures() {
    assert!(matches!(
        object(&"scalar", WireLimits::default()),
        Err(MaterializeError::NotObject)
    ));
}
