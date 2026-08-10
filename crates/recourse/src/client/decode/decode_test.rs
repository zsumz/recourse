//! Malformed and adversarial bounded JSON tests.

use serde_json::json;

use super::{DecodeError, DecodeLimit, DecodeLimits, decode_object};

#[test]
fn defaults_match_the_reviewed_client_budget() {
    let limits = DecodeLimits::default();
    let object = decode_object(br#"{"code":"NEW-9000","unknown":{"kept":true}}"#, limits)
        .unwrap_or_else(|error| panic!("bounded test object must decode: {error}"));

    assert_eq!(object["code"], "NEW-9000");
    assert_eq!(object["unknown"]["kept"], true);
}

#[test]
fn body_root_and_syntax_fail_without_panicking() {
    let limits = DecodeLimits::default().with_max_body_bytes(8);
    assert!(matches!(
        decode_object(br#"{"long":true}"#, limits),
        Err(DecodeError::LimitExceeded {
            limit: DecodeLimit::BodyBytes,
            ..
        })
    ));
    assert!(matches!(
        decode_object(b"[]", DecodeLimits::default()),
        Err(DecodeError::RootNotObject)
    ));
    assert!(matches!(
        decode_object(b"{", DecodeLimits::default()),
        Err(DecodeError::MalformedJson(_))
    ));
}

#[test]
fn every_tree_shape_budget_is_enforced() {
    assert_limit(
        &json!({"a": {"b": {}}}),
        DecodeLimits::default().with_max_nesting_depth(2),
        DecodeLimit::NestingDepth,
    );
    assert_limit(
        &json!({"a": 1, "b": 2}),
        DecodeLimits::default().with_max_object_properties(1),
        DecodeLimit::ObjectProperties,
    );
    assert_limit(
        &json!({"items": [1, 2]}),
        DecodeLimits::default().with_max_array_items(1),
        DecodeLimit::ArrayItems,
    );
    assert_limit(
        &json!({"value": "abcd"}),
        DecodeLimits::default().with_max_string_bytes(3),
        DecodeLimit::StringBytes,
    );
}

#[test]
fn semantic_arrays_have_stricter_independent_budgets() {
    assert_limit(
        &json!({"suggestions": ["one", "two"]}),
        DecodeLimits::default().with_max_suggestions(1),
        DecodeLimit::Suggestions,
    );
    assert_limit(
        &json!({"evidence": {"violations": [{}, {}]}}),
        DecodeLimits::default().with_max_violations(1),
        DecodeLimit::Violations,
    );
}

fn assert_limit(value: &serde_json::Value, limits: DecodeLimits, expected: DecodeLimit) {
    let body = serde_json::to_vec(value)
        .unwrap_or_else(|error| panic!("test value must serialize: {error}"));
    let result = decode_object(&body, limits);
    assert!(matches!(
        result,
        Err(DecodeError::LimitExceeded { limit, .. }) if limit == expected
    ));
}
