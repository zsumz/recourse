//! Closed JSON Schema format vocabulary enforced as runtime assertions.

use serde_json::{Map, Number, Value};

use super::{SchemaViolation, fail, number};

/// JSON Schema formats enforced as assertions by the Recourse evidence profile.
pub const SUPPORTED_SCHEMA_FORMATS: &[&str] = &[
    "date",
    "date-time",
    "duration",
    "email",
    "hostname",
    "idn-email",
    "idn-hostname",
    "ip",
    "ipv4",
    "ipv6",
    "iri",
    "iri-reference",
    "json-pointer",
    "regex",
    "relative-json-pointer",
    "time",
    "uri",
    "uri-reference",
    "uri-template",
    "uuid",
];

/// Schemars numeric representations retained beside enforced JSON types and bounds.
pub const SUPPORTED_SCHEMA_NUMERIC_FORMATS: &[&str] = &[
    "double", "float", "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64",
];

pub(super) fn validate(schema: &mut Map<String, Value>, path: &str) -> Result<(), SchemaViolation> {
    let Some(value) = schema.get("format") else {
        return Ok(());
    };
    let Some(format) = value.as_str() else {
        return fail(path, "format must be a string");
    };
    if SUPPORTED_SCHEMA_FORMATS.contains(&format) {
        return exact_type(schema.get("type"), "string", path, format);
    }
    let Some(representation) = numeric_representation(format) else {
        return fail(path, &format!("unsupported format {format:?}"));
    };
    exact_type(
        schema.get("type"),
        representation.schema_type(),
        path,
        format,
    )?;
    representation.apply_bounds(schema, path)
}

fn exact_type(
    value: Option<&Value>,
    expected: &str,
    path: &str,
    format: &str,
) -> Result<(), SchemaViolation> {
    let valid = match value {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => {
            values.len() == 2
                && values.iter().any(|value| value.as_str() == Some(expected))
                && values.iter().any(|value| value.as_str() == Some("null"))
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        fail(
            path,
            &format!("format {format:?} requires type {expected:?} with optional null"),
        )
    }
}

#[derive(Clone, Copy)]
enum NumericRepresentation {
    Integer { minimum: i128, maximum: i128 },
    Number { minimum: f64, maximum: f64 },
}

impl NumericRepresentation {
    const fn schema_type(self) -> &'static str {
        match self {
            Self::Integer { .. } => "integer",
            Self::Number { .. } => "number",
        }
    }

    fn apply_bounds(
        self,
        schema: &mut Map<String, Value>,
        path: &str,
    ) -> Result<(), SchemaViolation> {
        match self {
            Self::Integer { minimum, maximum } => {
                apply_integer_bound(schema, "minimum", minimum, true, path)?;
                apply_integer_bound(schema, "maximum", maximum, false, path)
            }
            Self::Number { minimum, maximum } => {
                apply_number_bound(schema, "minimum", minimum, true, path)?;
                apply_number_bound(schema, "maximum", maximum, false, path)
            }
        }
    }
}

fn numeric_representation(format: &str) -> Option<NumericRepresentation> {
    let integer = |minimum, maximum| NumericRepresentation::Integer { minimum, maximum };
    match format {
        "int8" => Some(integer(i8::MIN.into(), i8::MAX.into())),
        "int16" => Some(integer(i16::MIN.into(), i16::MAX.into())),
        "int32" => Some(integer(i32::MIN.into(), i32::MAX.into())),
        "int64" => Some(integer(i64::MIN.into(), i64::MAX.into())),
        "uint8" => Some(integer(0, u8::MAX.into())),
        "uint16" => Some(integer(0, u16::MAX.into())),
        "uint32" => Some(integer(0, u32::MAX.into())),
        "uint64" => Some(integer(0, i128::from(u64::MAX))),
        "float" => Some(NumericRepresentation::Number {
            minimum: public_f32(f32::MIN)?,
            maximum: public_f32(f32::MAX)?,
        }),
        "double" => Some(NumericRepresentation::Number {
            minimum: -f64::MAX,
            maximum: f64::MAX,
        }),
        _ => None,
    }
}

fn public_f32(value: f32) -> Option<f64> {
    value.to_string().parse().ok()
}

fn apply_integer_bound(
    schema: &mut Map<String, Value>,
    keyword: &str,
    representation: i128,
    lower: bool,
    path: &str,
) -> Result<(), SchemaViolation> {
    let boundary = integer_number(representation, path)?;
    let existing = schema.get(keyword).and_then(Value::as_number);
    if let Some(existing) = existing
        && !number::is_integer(existing, path)?
    {
        return fail(
            path,
            &format!("{keyword} for an integer format must be an exact integer"),
        );
    }
    let widening = match existing {
        Some(existing) => match number::compare(existing, &boundary, path)? {
            std::cmp::Ordering::Less => lower,
            std::cmp::Ordering::Greater => !lower,
            std::cmp::Ordering::Equal => false,
        },
        None => true,
    };
    if widening {
        schema.insert(keyword.to_owned(), Value::Number(boundary));
    }
    Ok(())
}

fn apply_number_bound(
    schema: &mut Map<String, Value>,
    keyword: &str,
    representation: f64,
    lower: bool,
    path: &str,
) -> Result<(), SchemaViolation> {
    let Some(boundary) = Number::from_f64(representation) else {
        return fail(path, "supported number format has a non-finite bound");
    };
    let existing = schema.get(keyword).and_then(Value::as_number);
    let widening = match existing {
        Some(existing) => match number::compare(existing, &boundary, path)? {
            std::cmp::Ordering::Less => lower,
            std::cmp::Ordering::Greater => !lower,
            std::cmp::Ordering::Equal => false,
        },
        None => true,
    };
    if widening {
        schema.insert(keyword.to_owned(), Value::Number(boundary));
    }
    Ok(())
}

fn integer_number(value: i128, path: &str) -> Result<Number, SchemaViolation> {
    match (i64::try_from(value), u64::try_from(value)) {
        (Ok(signed), _) => Ok(Number::from(signed)),
        (_, Ok(unsigned)) => Ok(Number::from(unsigned)),
        _ => fail(
            path,
            "supported integer format has an unrepresentable bound",
        ),
    }
}
