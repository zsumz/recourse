//! Public numeric-emitter domain boundaries.

use serde_json::Number;

use super::{is_public, values_equal};

fn number(encoded: &str) -> Number {
    serde_json::from_str(encoded).unwrap_or_else(|error| panic!("exact number must parse: {error}"))
}

#[test]
fn public_domain_is_i64_u64_and_exact_finite_float_emission() {
    for encoded in [
        "-9223372036854775808",
        "18446744073709551615",
        "0.1",
        "3.4028235e38",
        "1.7976931348623157e308",
    ] {
        assert!(
            is_public(&number(encoded), "$").unwrap_or(false),
            "{encoded}"
        );
    }

    for encoded in ["18446744073709551616", "0.100000000000000000001", "1e400"] {
        assert!(
            !is_public(&number(encoded), "$").unwrap_or(true),
            "{encoded}"
        );
    }
}

#[test]
fn exact_equality_ignores_equivalent_decimal_spelling() {
    let integer = serde_json::Value::Number(number("1"));
    let decimal = serde_json::Value::Number(number("1.00"));
    assert!(values_equal(&integer, &decimal));
}
