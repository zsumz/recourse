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

struct WideInteger;

impl Serialize for WideInteger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("value", &u128::MAX)?;
        map.end()
    }
}

#[derive(Serialize)]
struct FloatValue<T> {
    value: T,
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

#[test]
fn numbers_that_serde_json_would_rewrite_fail_before_materialization() {
    for result in [
        object(&WideInteger, WireLimits::default()),
        object(
            &FloatValue {
                value: Some(f32::NAN),
            },
            WireLimits::default(),
        ),
        object(
            &FloatValue {
                value: Some(f64::INFINITY),
            },
            WireLimits::default(),
        ),
    ] {
        assert!(matches!(result, Err(MaterializeError::Json(_))));
    }
}

#[test]
fn finite_float_boundaries_retain_their_public_json_values() {
    for value in [f32::MIN, f32::MAX] {
        let actual = object(&FloatValue { value }, WireLimits::default())
            .unwrap_or_else(|error| panic!("finite float must materialize: {error:?}"));
        let expected = serde_json::from_slice::<serde_json::Value>(
            serde_json::to_string(&FloatValue { value })
                .unwrap_or_else(|error| panic!("fixture must encode: {error}"))
                .as_bytes(),
        )
        .unwrap_or_else(|error| panic!("fixture must parse: {error}"));
        assert_eq!(actual, expected);
    }
}

#[test]
fn finite_f64_bit_corpus_round_trips_without_mutation() {
    let corpus = [
        0x0000_0000_0000_0000,
        0x8000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x8000_0000_0000_0001,
        0x000f_ffff_ffff_ffff,
        0x0010_0000_0000_0000,
        0x3fd5_5555_5555_5555,
        0x3fef_ffff_ffff_ffff,
        0x3ff0_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        0x4340_0000_0000_0000,
        0x4340_0000_0000_0001,
        0x44b5_2d02_c7e1_4af6,
        0x7fef_ffff_ffff_ffff,
        0xffef_ffff_ffff_ffff,
    ];
    for bits in corpus {
        let value = f64::from_bits(bits);
        let actual = object(&FloatValue { value }, WireLimits::default())
            .unwrap_or_else(|error| panic!("finite f64 must materialize: {error:?}"));
        let reparsed = actual["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("materialized f64 must remain numeric: {actual}"));
        assert_eq!(
            reparsed.to_bits(),
            bits,
            "f64 bits changed for serialized value {value}"
        );
    }
}
