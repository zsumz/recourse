//! Focused tests for canonical diagnostic code identity.

use std::str::FromStr;

use super::{Code, CodeNumber, CodeNumberError, CodeParseError};

#[test]
fn code_number_is_positive_and_serializes_as_an_integer() {
    let number = CodeNumber::new(1004);

    assert_eq!(number.get(), 1004);
    assert_eq!(number.to_string(), "1004");
    assert!(matches!(
        serde_json::to_string(&number),
        Ok(value) if value == "1004"
    ));
    assert!(matches!(
        serde_json::from_str::<CodeNumber>("1004"),
        Ok(value) if value == number
    ));
    assert_eq!(CodeNumber::try_new(0), Err(CodeNumberError));
    assert!(serde_json::from_str::<CodeNumber>("0").is_err());
}

#[test]
fn code_round_trips_through_text_and_json() {
    let code = Code::new("DSP", CodeNumber::new(1004));
    let expected = Code::from_str("DSP-1004");

    assert_eq!(code, expected);
    assert_eq!(
        expected.as_ref().map(ToString::to_string),
        Ok("DSP-1004".to_owned())
    );
    let Some(expected) = expected.ok() else {
        return;
    };
    assert!(matches!(
        serde_json::to_string(&expected),
        Ok(value) if value == "\"DSP-1004\""
    ));
    assert_eq!(
        serde_json::from_str::<Code>("\"DSP-1004\"").ok(),
        Some(expected)
    );
}

#[test]
fn code_exposes_its_validated_parts() {
    let code = Code::from_str("DSP9-42");

    assert_eq!(code.as_ref().map(Code::prefix), Ok("DSP9"));
    assert_eq!(code.as_ref().map(Code::number), Ok(CodeNumber::new(42)));
}

#[test]
fn parser_rejects_noncanonical_numbers() {
    for value in ["DSP-", "DSP-0", "DSP-01004", "DSP-1.0", "DSP--1"] {
        assert!(Code::from_str(value).is_err(), "accepted {value}");
    }
}

#[test]
fn parser_rejects_invalid_prefixes() {
    for value in [
        "D-1004",
        "DISPATCHER-1004",
        "9DSP-1004",
        "dsp-1004",
        "DS_P-1004",
    ] {
        assert!(Code::from_str(value).is_err(), "accepted {value}");
    }
}

#[test]
fn parser_reports_a_missing_separator() {
    assert!(matches!(
        Code::from_str("DSP1004"),
        Err(CodeParseError::MissingSeparator)
    ));
}

#[test]
fn parser_rejects_numbers_larger_than_u32() {
    assert!(matches!(
        Code::from_str("DSP-4294967296"),
        Err(CodeParseError::Number(_))
    ));
}
