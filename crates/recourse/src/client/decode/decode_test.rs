//! Malformed and adversarial bounded JSON tests.

use serde_json::json;

use super::{DecodeError, DecodeLimit, DecodeLimits, decode_embedded_object, decode_object};

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
fn duplicate_members_are_rejected_at_every_depth() {
    for body in [
        br#"{"code":"CLI-1","code":"CLI-2"}"#.as_slice(),
        br#"{"evidence":{"id":1,"id":2}}"#.as_slice(),
        br#"{"evidence":{"id":1,"\u0069d":2}}"#.as_slice(),
        br#"{"evidence":[{"id":1,"id":2}]}"#.as_slice(),
    ] {
        let error = decode_object(body, DecodeLimits::default())
            .err()
            .unwrap_or_else(|| panic!("duplicate member must be rejected"));
        assert!(matches!(error, DecodeError::MalformedJson(_)));
        assert!(error.to_string().contains("duplicate JSON member"));
    }
}

#[test]
fn numbers_and_private_key_objects_retain_their_wire_shape() {
    let object = decode_object(
        br#"{"decimal":1.25,"exponent":1e-30,"wide":18446744073709551616,"opaque":{"$serde_json::private::Number":"1.25"}}"#,
        DecodeLimits::default(),
    )
    .unwrap_or_else(|error| panic!("numeric fixture must decode: {error}"));

    for (member, expected) in [
        ("decimal", "1.25"),
        ("exponent", "1e-30"),
        ("wide", "18446744073709551616"),
    ] {
        assert!(object[member].is_number());
        assert_eq!(object[member].to_string(), expected);
    }
    assert_eq!(object["opaque"]["$serde_json::private::Number"], "1.25");
}

#[test]
fn malformed_scalar_and_container_boundaries_are_rejected() {
    for body in [
        br#"{"value":01}"#.as_slice(),
        br#"{"value":1.}"#.as_slice(),
        br#"{"value":true false}"#.as_slice(),
        br#"{"value":"unterminated}"#.as_slice(),
        br#"{"value":[],}"#.as_slice(),
        br#"{"value":[1 2]}"#.as_slice(),
        br#"{"value":1} trailing"#.as_slice(),
    ] {
        assert!(matches!(
            decode_object(body, DecodeLimits::default()),
            Err(DecodeError::MalformedJson(_))
        ));
    }
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
fn numeric_tokens_have_an_independent_parse_budget() {
    let limits = DecodeLimits::default().with_max_number_bytes(4);
    assert!(matches!(
        decode_object(br#"{"value":12345x}"#, limits),
        Err(DecodeError::LimitExceeded {
            limit: DecodeLimit::NumberBytes,
            maximum: 4,
            actual: 6,
        })
    ));
}

#[test]
fn structural_limits_precede_invalid_descendant_construction() {
    for (body, limits, expected) in [
        (
            br#"{"blocked":not-json}"#.as_slice(),
            DecodeLimits::default().with_max_object_properties(0),
            DecodeLimit::ObjectProperties,
        ),
        (
            br#"{"items":[not-json]}"#.as_slice(),
            DecodeLimits::default().with_max_array_items(0),
            DecodeLimit::ArrayItems,
        ),
        (
            br#"{"nested":{not-json}}"#.as_slice(),
            DecodeLimits::default().with_max_nesting_depth(1),
            DecodeLimit::NestingDepth,
        ),
        (
            br#"{"long":not-json}"#.as_slice(),
            DecodeLimits::default().with_max_string_bytes(3),
            DecodeLimit::StringBytes,
        ),
    ] {
        assert!(matches!(
            decode_object(body, limits),
            Err(DecodeError::LimitExceeded { limit, .. }) if limit == expected
        ));
    }
}

#[test]
fn embedded_roots_reserve_their_enclosing_envelope_depth() {
    let limits = DecodeLimits::default().with_max_nesting_depth(1);
    assert!(decode_object(b"{}", limits).is_ok());
    assert!(matches!(
        decode_embedded_object(b"{}", limits),
        Err(DecodeError::LimitExceeded {
            limit: DecodeLimit::NestingDepth,
            ..
        })
    ));
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
