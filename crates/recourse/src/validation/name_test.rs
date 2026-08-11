//! Focused tests for validation location names.

use super::{HeaderName, HeaderNameError, ParameterName, ParameterNameError};

#[test]
fn parameter_names_are_bounded_public_text() {
    assert_eq!(
        ParameterName::new("destination")
            .as_ref()
            .map(ParameterName::as_str),
        Ok("destination")
    );
    assert_eq!(ParameterName::new(""), Err(ParameterNameError::Empty));
    assert!(matches!(
        ParameterName::new("line\n"),
        Err(ParameterNameError::ControlCharacter { .. })
    ));
    assert!(matches!(
        ParameterName::new("x".repeat(129)),
        Err(ParameterNameError::TooLong { actual_chars: 129 })
    ));
}

#[test]
fn parameter_schema_and_runtime_agree_for_multibyte_and_control_text() {
    let schema = schemars::schema_for!(ParameterName).to_value();
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|error| panic!("ParameterName schema must compile: {error}"));
    for value in ["é".repeat(128), "é".repeat(129), "line\nfeed".into()] {
        assert_eq!(
            ParameterName::new(value.clone()).is_ok(),
            validator.is_valid(&serde_json::json!(value))
        );
    }
}

#[test]
fn header_names_are_validated_and_canonicalized() {
    assert_eq!(
        HeaderName::new("X-Request-ID")
            .as_ref()
            .map(HeaderName::as_str),
        Ok("x-request-id")
    );
    assert_eq!(HeaderName::new("bad header"), Err(HeaderNameError));
    assert!(serde_json::from_str::<HeaderName>("\"bad header\"").is_err());
}
