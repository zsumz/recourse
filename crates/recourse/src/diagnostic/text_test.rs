//! Focused tests for bounded dynamic caller-visible prose.

use schemars::schema_for;

use super::{DEFAULT_PUBLIC_TEXT_BYTES, PublicText, PublicTextError};

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
        PublicText::with_max_bytes("four", 3),
        Err(PublicTextError::TooLong {
            actual_bytes: 4,
            max_bytes: 3,
        })
    );
    assert_eq!(
        PublicText::new("line one\nline two"),
        Err(PublicTextError::ControlCharacter { character_index: 8 })
    );
    assert_eq!(
        PublicText::with_max_bytes("text", 0),
        Err(PublicTextError::ZeroLimit)
    );
}

#[test]
fn byte_budget_is_explicit_for_multibyte_text() {
    assert!(PublicText::with_max_bytes("é", 1).is_err());
    assert!(PublicText::with_max_bytes("é", 2).is_ok());
}

#[test]
fn generated_schema_carries_default_bounds() {
    let schema = schema_for!(PublicText).to_value();

    assert_eq!(schema.get("minLength"), Some(&serde_json::json!(1)));
    assert_eq!(
        schema.get("maxLength"),
        Some(&serde_json::json!(DEFAULT_PUBLIC_TEXT_BYTES))
    );
}
