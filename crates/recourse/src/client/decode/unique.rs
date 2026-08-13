//! Feature-invariant JSON parsing with duplicate object-member rejection.

use serde::de::Error as _;
use serde_json::{Map, Number, Value};

use super::{DecodeError, DecodeLimit, DecodeLimits};

const MAX_PARSER_NESTING_DEPTH: usize = 128;

pub(super) fn parse(
    body: &[u8],
    limits: DecodeLimits,
    initial_depth: usize,
) -> Result<Value, DecodeError> {
    Parser::new(body, limits).parse(initial_depth)
}

struct Parser<'a> {
    body: &'a [u8],
    cursor: usize,
    limits: DecodeLimits,
}

impl<'a> Parser<'a> {
    const fn new(body: &'a [u8], limits: DecodeLimits) -> Self {
        Self {
            body,
            cursor: 0,
            limits,
        }
    }

    fn parse(mut self, initial_depth: usize) -> Result<Value, DecodeError> {
        let value = self.value(initial_depth)?;
        self.whitespace();
        if self.cursor != self.body.len() {
            return self.fail("trailing characters after JSON value");
        }
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Value, DecodeError> {
        self.whitespace();
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.fail("expected a JSON value"),
            None => self.fail("unexpected end of JSON input"),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, DecodeError> {
        self.nested(depth)?;
        self.cursor += 1;
        self.whitespace();
        let mut object = Map::new();
        if self.consume(b'}') {
            return Ok(Value::Object(object));
        }
        loop {
            Self::enforce(
                DecodeLimit::ObjectProperties,
                self.limits.max_object_properties(),
                object.len() + 1,
            )?;
            if self.peek() != Some(b'"') {
                return self.fail("object member name must be a string");
            }
            let name = self.string()?;
            if object.contains_key(&name) {
                return self.fail(&format!("duplicate JSON member `{name}`"));
            }
            self.whitespace();
            self.require(b':', "expected ':' after object member name")?;
            let value = self.value(depth + 1)?;
            object.insert(name, value);
            self.whitespace();
            if self.consume(b'}') {
                return Ok(Value::Object(object));
            }
            self.require(b',', "expected ',' or '}' after object member")?;
            self.whitespace();
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, DecodeError> {
        self.nested(depth)?;
        self.cursor += 1;
        self.whitespace();
        let mut array = Vec::new();
        if self.consume(b']') {
            return Ok(Value::Array(array));
        }
        loop {
            Self::enforce(
                DecodeLimit::ArrayItems,
                self.limits.max_array_items(),
                array.len() + 1,
            )?;
            array.push(self.value(depth + 1)?);
            self.whitespace();
            if self.consume(b']') {
                return Ok(Value::Array(array));
            }
            self.require(b',', "expected ',' or ']' after array item")?;
            self.whitespace();
        }
    }

    fn string(&mut self) -> Result<String, DecodeError> {
        let start = self.cursor;
        self.cursor += 1;
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.cursor += 1;
                    let value: String = serde_json::from_slice(&self.body[start..self.cursor])
                        .map_err(DecodeError::MalformedJson)?;
                    Self::enforce(
                        DecodeLimit::StringBytes,
                        self.limits.max_string_bytes(),
                        value.len(),
                    )?;
                    return Ok(value);
                }
                b'\\' => {
                    self.cursor += 1;
                    if self.peek().is_none() {
                        return self.fail("unterminated JSON string escape");
                    }
                    self.cursor += 1;
                }
                _ => self.cursor += 1,
            }
        }
        self.fail("unterminated JSON string")
    }

    fn number(&mut self) -> Result<Value, DecodeError> {
        let start = self.cursor;
        while self.peek().is_some_and(|byte| !is_value_delimiter(byte)) {
            self.cursor += 1;
        }
        Self::enforce(
            DecodeLimit::NumberBytes,
            self.limits.max_number_bytes(),
            self.cursor - start,
        )?;
        serde_json::from_slice::<Number>(&self.body[start..self.cursor])
            .map(Value::Number)
            .map_err(DecodeError::MalformedJson)
    }

    fn literal(&mut self, token: &[u8], value: Value) -> Result<Value, DecodeError> {
        if self.body[self.cursor..].starts_with(token) {
            self.cursor += token.len();
            Ok(value)
        } else {
            self.fail("invalid JSON literal")
        }
    }

    fn nested(&self, depth: usize) -> Result<(), DecodeError> {
        Self::enforce(
            DecodeLimit::NestingDepth,
            self.limits.max_nesting_depth(),
            depth + 1,
        )?;
        if depth >= MAX_PARSER_NESTING_DEPTH {
            self.fail("JSON recursion limit exceeded")
        } else {
            Ok(())
        }
    }

    fn whitespace(&mut self) {
        while self.peek().is_some_and(is_whitespace) {
            self.cursor += 1;
        }
    }

    fn require(&mut self, byte: u8, message: &str) -> Result<(), DecodeError> {
        if self.consume(byte) {
            Ok(())
        } else {
            self.fail(message)
        }
    }

    fn consume(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.body.get(self.cursor).copied()
    }

    fn enforce(limit: DecodeLimit, maximum: usize, actual: usize) -> Result<(), DecodeError> {
        if actual > maximum {
            Err(DecodeError::LimitExceeded {
                limit,
                maximum,
                actual,
            })
        } else {
            Ok(())
        }
    }

    fn fail<T>(&self, message: &str) -> Result<T, DecodeError> {
        Err(DecodeError::MalformedJson(serde_json::Error::custom(
            format!("{message} at byte {}", self.cursor),
        )))
    }
}

const fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
}

const fn is_value_delimiter(byte: u8) -> bool {
    is_whitespace(byte) || matches!(byte, b',' | b']' | b'}')
}
