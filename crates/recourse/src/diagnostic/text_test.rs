//! Focused tests for bounded dynamic caller-visible prose.

use schemars::schema_for;

use super::{DEFAULT_PUBLIC_TEXT_CHARS, PublicText, PublicTextError};

#[test]
fn public_text_round_trips_as_a_json_string() {
    let text = PublicText::new("Check the job identifier.");
    let Some(text) = text.ok() else {
        return;
    };

    assert_eq!(text.as_str(), "Check the job identifier.");
    assert!(matches!(
        serde_json::to_string(&text),
        Ok(value) if value == "\"Check the job identifier.\""
    ));
    assert_eq!(
        serde_json::from_str::<PublicText>("\"Check the job identifier.\"").ok(),
        Some(text)
    );
}

#[test]
fn text_rejects_empty_overlong_and_control_values() {
    assert_eq!(PublicText::new(""), Err(PublicTextError::Empty));
    assert_eq!(
        PublicText::with_max_chars("four", 3),
        Err(PublicTextError::TooLong {
            actual_chars: 4,
            max_chars: 3,
        })
    );
    assert_eq!(
        PublicText::new("line one\nline two"),
        Err(PublicTextError::ControlCharacter { character_index: 8 })
    );
    assert_eq!(
        PublicText::with_max_chars("text", 0),
        Err(PublicTextError::ZeroLimit)
    );
}

#[test]
fn character_budget_accepts_multibyte_text_without_schema_drift() {
    assert!(PublicText::with_max_chars("é", 1).is_ok());
    assert!(PublicText::with_max_chars("éé", 1).is_err());
}

#[test]
fn generated_schema_carries_default_bounds() {
    let schema = schema_for!(PublicText).to_value();

    assert_eq!(schema.get("minLength"), Some(&serde_json::json!(1)));
    assert_eq!(
        schema.get("maxLength"),
        Some(&serde_json::json!(DEFAULT_PUBLIC_TEXT_CHARS))
    );
}

#[test]
fn generated_schema_and_runtime_agree_at_text_boundaries() {
    let schema = schema_for!(PublicText).to_value();
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|error| panic!("PublicText schema must compile: {error}"));
    let ascii_limit = "a".repeat(DEFAULT_PUBLIC_TEXT_CHARS);
    let multibyte_limit = "é".repeat(DEFAULT_PUBLIC_TEXT_CHARS);
    let over_limit = "é".repeat(DEFAULT_PUBLIC_TEXT_CHARS + 1);

    for value in [
        ascii_limit,
        multibyte_limit,
        over_limit,
        "line\nfeed".into(),
    ] {
        assert_eq!(
            PublicText::new(value.clone()).is_ok(),
            validator.is_valid(&serde_json::json!(value))
        );
    }
}
