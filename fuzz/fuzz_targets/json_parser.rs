//! Differentially fuzzes Recourse parsing against Serde JSON semantics.
#![no_main]

use http::{HeaderMap, StatusCode};
use libfuzzer_sys::fuzz_target;
use recourse::client::{DecodeLimits, ReceivedProblem};
use serde_json::Value;

fuzz_target!(|body: &[u8]| {
    let Ok(Value::Object(reference)) = serde_json::from_slice::<Value>(body) else {
        return;
    };

    if let Ok(parsed) = decode(body, body.len()) {
        assert_eq!(parsed.raw(), &reference);
    }

    let canonical = serde_json::to_vec(&Value::Object(reference.clone())).unwrap_or_default();
    let parsed = decode(&canonical, canonical.len())
        .unwrap_or_else(|error| panic!("canonical Serde JSON object was rejected: {error}"));
    assert_eq!(parsed.raw(), &reference);
});

fn decode(body: &[u8], size: usize) -> Result<ReceivedProblem, recourse::client::DecodeError> {
    let maximum = size.saturating_add(1);
    let limits = DecodeLimits::default()
        .with_max_body_bytes(maximum)
        .with_max_nesting_depth(maximum)
        .with_max_object_properties(maximum)
        .with_max_array_items(maximum)
        .with_max_string_bytes(maximum)
        .with_max_number_bytes(maximum)
        .with_max_suggestions(maximum)
        .with_max_violations(maximum);
    ReceivedProblem::from_slice(StatusCode::BAD_GATEWAY, &HeaderMap::new(), body, limits)
}
