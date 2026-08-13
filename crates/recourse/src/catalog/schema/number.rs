//! Exact comparison for the complete JSON decimal number domain.

mod emission;

use std::cmp::Ordering;

use num_bigint::BigInt;
use serde_json::{Number, Value};

use super::SchemaViolation;
use crate::wire::WireLimits;

pub(crate) use emission::{is_public, value_is_public};
pub(crate) use emission::{unordered_values_equal, values_equal};

#[derive(Debug)]
struct ExactNumber {
    negative: bool,
    significand: Box<[u8]>,
    magnitude: BigInt,
}

impl ExactNumber {
    fn parse(number: &Number, path: &str) -> Result<Self, SchemaViolation> {
        validate_token(number, path)?;
        let encoded = number.as_str();
        let (negative, unsigned) = encoded
            .strip_prefix('-')
            .map_or((false, encoded), |value| (true, value));
        let (mantissa, exponent) = split_exponent(unsigned);
        let decimal = mantissa.find('.').unwrap_or(mantissa.len());
        let digits = mantissa
            .bytes()
            .filter(u8::is_ascii_digit)
            .collect::<Vec<_>>();
        let Some(first) = digits.iter().position(|digit| *digit != b'0') else {
            return Ok(Self {
                negative: false,
                significand: Box::new([]),
                magnitude: BigInt::from(0_u8),
            });
        };
        let last = digits
            .iter()
            .rposition(|digit| *digit != b'0')
            .unwrap_or(first);
        let exponent = parse_exponent(exponent).ok_or_else(|| SchemaViolation {
            path: path.to_owned(),
            reason: "numeric keyword could not be represented exactly".to_owned(),
        })?;
        let magnitude = exponent + BigInt::from(decimal) - BigInt::from(first) - 1_u8;
        Ok(Self {
            negative,
            significand: digits[first..=last].into(),
            magnitude,
        })
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.is_zero() {
            return if other.is_zero() {
                Ordering::Equal
            } else if other.negative {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        if other.is_zero() {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => self.compare_absolute(other),
            (true, true) => self.compare_absolute(other).reverse(),
        }
    }

    fn compare_absolute(&self, other: &Self) -> Ordering {
        self.magnitude.cmp(&other.magnitude).then_with(|| {
            let length = self.significand.len().max(other.significand.len());
            (0..length)
                .map(|index| {
                    self.significand
                        .get(index)
                        .copied()
                        .unwrap_or(b'0')
                        .cmp(&other.significand.get(index).copied().unwrap_or(b'0'))
                })
                .find(|ordering| *ordering != Ordering::Equal)
                .unwrap_or(Ordering::Equal)
        })
    }

    fn is_zero(&self) -> bool {
        self.significand.is_empty()
    }

    fn is_integer(&self) -> bool {
        self.is_zero() || self.magnitude >= BigInt::from(self.significand.len() - 1)
    }
}

pub(super) fn compare(
    left: &Number,
    right: &Number,
    path: &str,
) -> Result<Ordering, SchemaViolation> {
    Ok(ExactNumber::parse(left, path)?.compare(&ExactNumber::parse(right, path)?))
}

pub(super) fn is_integer(number: &Number, path: &str) -> Result<bool, SchemaViolation> {
    Ok(ExactNumber::parse(number, path)?.is_integer())
}

pub(super) fn has_primitive_integer_emitter(
    number: &Number,
    path: &str,
) -> Result<bool, SchemaViolation> {
    let value = ExactNumber::parse(number, path)?;
    if !value.is_integer() {
        return Ok(false);
    }
    let minimum = ExactNumber::parse(&Number::from(i64::MIN), path)?;
    let maximum = ExactNumber::parse(&Number::from(u64::MAX), path)?;
    Ok(value.compare(&minimum) != Ordering::Less && value.compare(&maximum) != Ordering::Greater)
}

pub(super) fn is_positive(number: &Number, path: &str) -> Result<bool, SchemaViolation> {
    let zero = Number::from(0_u8);
    Ok(compare(number, &zero, path)? == Ordering::Greater)
}

pub(super) fn validate_tokens(value: &Value) -> Result<(), SchemaViolation> {
    let mut pending = vec![(value, "$".to_owned())];
    while let Some((value, path)) = pending.pop() {
        match value {
            Value::Number(number) => validate_token(number, &path)?,
            Value::Array(values) => pending.extend(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (value, format!("{path}/{index}"))),
            ),
            Value::Object(object) => pending.extend(
                object
                    .iter()
                    .map(|(key, value)| (value, format!("{path}/{}", pointer_segment(key)))),
            ),
            Value::Null | Value::Bool(_) | Value::String(_) => {}
        }
    }
    Ok(())
}

fn validate_token(number: &Number, path: &str) -> Result<(), SchemaViolation> {
    let actual = number.as_str().len();
    let maximum = WireLimits::DEFAULT_MAX_NUMBER_BYTES;
    if actual > maximum {
        Err(SchemaViolation {
            path: path.to_owned(),
            reason: format!("numeric token is {actual} bytes; default wire maximum is {maximum}"),
        })
    } else {
        Ok(())
    }
}

fn pointer_segment(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn split_exponent(value: &str) -> (&str, &str) {
    value
        .find(['e', 'E'])
        .map_or((value, "0"), |index| (&value[..index], &value[index + 1..]))
}

fn parse_exponent(value: &str) -> Option<BigInt> {
    let (negative, digits) = value
        .strip_prefix('-')
        .map_or((false, value), |digits| (true, digits));
    let digits = digits.strip_prefix('+').unwrap_or(digits);
    if digits.is_empty() || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let mut exponent = BigInt::from(0_u8);
    for digit in digits.bytes() {
        exponent = exponent * 10_u8 + (digit - b'0');
    }
    Some(if negative { -exponent } else { exponent })
}
