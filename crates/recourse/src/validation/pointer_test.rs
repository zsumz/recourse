//! Focused tests for public JSON Pointer validation.

use super::{JsonPointer, JsonPointerError};

#[test]
fn root_and_escaped_pointers_are_valid() {
    for value in ["", "/destination", "/a~1b", "/m~0n", "/0/é"] {
        assert_eq!(
            JsonPointer::new(value).as_ref().map(JsonPointer::as_str),
            Ok(value)
        );
    }
}

#[test]
fn malformed_pointers_are_rejected_precisely() {
    assert_eq!(
        JsonPointer::new("destination"),
        Err(JsonPointerError::MissingRootSeparator)
    );
    assert_eq!(
        JsonPointer::new("/bad~2escape"),
        Err(JsonPointerError::InvalidEscape { byte_index: 4 })
    );
    assert_eq!(
        JsonPointer::new("/line\n"),
        Err(JsonPointerError::ControlCharacter { character_index: 5 })
    );
}

#[test]
fn pointer_json_decoding_revalidates_input() {
    assert!(serde_json::from_str::<JsonPointer>("\"/valid\"").is_ok());
    assert!(serde_json::from_str::<JsonPointer>("\"invalid\"").is_err());
}

#[test]
fn pointer_schema_and_runtime_both_reject_controls() {
    let schema = schemars::schema_for!(JsonPointer).to_value();
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|error| panic!("JsonPointer schema must compile: {error}"));
    for value in ["", "/é", "/line\nfeed", "/bad~2escape"] {
        assert_eq!(
            JsonPointer::new(value).is_ok(),
            validator.is_valid(&serde_json::json!(value))
        );
    }
}
