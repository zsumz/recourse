//! Literal protocol values validate at compile time and agree with runtime.
//!
//! Every constant below is evaluated while this test compiles, so an invalid
//! literal is a build failure rather than an unreachable runtime error arm.

use recourse::{
    dependencies::http::{Method, header::ALLOW},
    diagnostic::PublicText,
    http::{AllowedMethods, BearerChallenge, HttpPolicy, MethodNotAllowed},
    validation::{HeaderName, JsonPointer, ParameterName},
};

const DETAIL: PublicText = PublicText::from_static("Provide a nonempty destination.");
const BODY_POINTER: JsonPointer = JsonPointer::from_static("/destination");
const ROOT_POINTER: JsonPointer = JsonPointer::from_static("");
const ESCAPED_POINTER: JsonPointer = JsonPointer::from_static("/a~0b/c~1d");
const PARAMETER: ParameterName = ParameterName::from_static("job_id");
const FIELD: HeaderName = HeaderName::from_static("idempotency-key");
const CHALLENGE: BearerChallenge = BearerChallenge::from_static("dispatch");
const ALLOWED: AllowedMethods = AllowedMethods::from_static(&[Method::POST]);

#[test]
fn literal_text_and_locations_equal_their_validated_values() {
    assert_eq!(
        PublicText::new("Provide a nonempty destination.").ok(),
        Some(DETAIL)
    );
    assert_eq!(JsonPointer::new("/destination").ok(), Some(BODY_POINTER));
    assert_eq!(JsonPointer::new("").ok(), Some(ROOT_POINTER));
    assert_eq!(JsonPointer::new("/a~0b/c~1d").ok(), Some(ESCAPED_POINTER));
    assert_eq!(ParameterName::new("job_id").ok(), Some(PARAMETER));
}

#[test]
fn literal_field_names_match_runtime_canonicalization() {
    assert_eq!(HeaderName::new("Idempotency-Key").ok(), Some(FIELD));
    assert_eq!(FIELD.as_str(), "idempotency-key");
}

/// Pins the byte cap `HeaderName::from_static` asserts while compiling.
///
/// The literal constructor cannot be handed a 65536-byte literal in a readable
/// test, so this asserts the runtime boundary its `const` assert mirrors: a
/// literal above this length would construct a value `new` rejects, breaking
/// the round trip through this type's own `Deserialize`.
#[test]
fn the_literal_field_name_cap_is_the_runtime_field_name_cap() {
    let longest = "a".repeat(65_535);
    let too_long = "a".repeat(65_536);

    assert_eq!(
        HeaderName::new(&longest)
            .ok()
            .as_ref()
            .map(HeaderName::as_str),
        Some(longest.as_str())
    );
    assert!(HeaderName::new(&too_long).is_err());
}

#[test]
fn literal_policy_inputs_equal_their_validated_values() {
    assert_eq!(BearerChallenge::new("dispatch").ok(), Some(CHALLENGE));
    assert_eq!(AllowedMethods::new([Method::POST]).ok(), Some(ALLOWED));
}

#[test]
fn a_literal_method_set_still_emits_a_sorted_deduplicated_allow_header() {
    let repeated = AllowedMethods::from_static(&[Method::POST, Method::GET, Method::POST]);
    let headers = MethodNotAllowed::headers(repeated);

    assert_eq!(
        headers.ok().and_then(|value| value.get(ALLOW).cloned()),
        Some(recourse::dependencies::http::HeaderValue::from_static(
            "GET, POST"
        ))
    );
}

#[test]
fn literal_text_serializes_exactly_like_validated_text() {
    let encoded = serde_json::to_string(&DETAIL);

    assert_eq!(
        encoded.ok().as_deref(),
        Some("\"Provide a nonempty destination.\"")
    );
}
