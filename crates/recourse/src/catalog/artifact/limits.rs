//! One resource profile shared by catalog generation and parsing.

use crate::wire::WireLimits;

use super::MAX_CATALOG_ARTIFACT_BYTES;

const MAX_NESTING_DEPTH: usize = 64;
const MAX_OBJECT_PROPERTIES: usize = 16_384;
const MAX_ARRAY_ITEMS: usize = 16_384;
pub(crate) const MAX_ARTIFACT_STRING_BYTES: usize = 512 * 1024;

pub(crate) fn artifact_limits() -> WireLimits {
    WireLimits::default()
        .with_max_body_bytes(MAX_CATALOG_ARTIFACT_BYTES)
        .with_max_nesting_depth(MAX_NESTING_DEPTH)
        .with_max_object_properties(MAX_OBJECT_PROPERTIES)
        .with_max_array_items(MAX_ARRAY_ITEMS)
        .with_max_string_bytes(MAX_ARTIFACT_STRING_BYTES)
}
