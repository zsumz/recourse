//! Feature-invariant JSON parsing with duplicate object-member rejection.

use serde::de::Error as _;
use serde_json::{Map, Number, Value};

const MAX_PARSER_NESTING_DEPTH: usize = 128;

pub(super) fn parse(body: &[u8]) -> Result<Value, serde_json::Error> {
    Parser::new(body).parse()
}

struct Parser<'a> {
    body: &'a [u8],
    cursor: usize,
}

impl<'a> Parser<'a> {
    const fn new(body: &'a [u8]) -> Self {
        Self { body, cursor: 0 }
    }

    fn parse(mut self) -> Result<Value, serde_json::Error> {
        let value = self.value(0)?;
        self.whitespace();
        if self.cursor != self.body.len() {
            return self.fail("trailing characters after JSON value");
        }
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Value, serde_json::Error> {
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

    fn object(&mut self, depth: usize) -> Result<Value, serde_json::Error> {
        Self::nested(depth)?;
        self.cursor += 1;
        self.whitespace();
        let mut object = Map::new();
        if self.consume(b'}') {
            return Ok(Value::Object(object));
        }
        loop {
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

    fn array(&mut self, depth: usize) -> Result<Value, serde_json::Error> {
        Self::nested(depth)?;
        self.cursor += 1;
        self.whitespace();
        let mut array = Vec::new();
        if self.consume(b']') {
            return Ok(Value::Array(array));
        }
        loop {
            array.push(self.value(depth + 1)?);
            self.whitespace();
            if self.consume(b']') {
                return Ok(Value::Array(array));
            }
            self.require(b',', "expected ',' or ']' after array item")?;
            self.whitespace();
        }
    }

    fn string(&mut self) -> Result<String, serde_json::Error> {
        let start = self.cursor;
        self.cursor += 1;
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.cursor += 1;
                    return serde_json::from_slice(&self.body[start..self.cursor]);
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

    fn number(&mut self) -> Result<Value, serde_json::Error> {
        let start = self.cursor;
        while self.peek().is_some_and(|byte| !is_value_delimiter(byte)) {
            self.cursor += 1;
        }
        serde_json::from_slice::<Number>(&self.body[start..self.cursor]).map(Value::Number)
    }

    fn literal(&mut self, token: &[u8], value: Value) -> Result<Value, serde_json::Error> {
        if self.body[self.cursor..].starts_with(token) {
            self.cursor += token.len();
            Ok(value)
        } else {
            self.fail("invalid JSON literal")
        }
    }

    fn nested(depth: usize) -> Result<(), serde_json::Error> {
        if depth >= MAX_PARSER_NESTING_DEPTH {
            Err(serde_json::Error::custom("JSON recursion limit exceeded"))
        } else {
            Ok(())
        }
    }

    fn whitespace(&mut self) {
        while self.peek().is_some_and(is_whitespace) {
            self.cursor += 1;
        }
    }

    fn require(&mut self, byte: u8, message: &str) -> Result<(), serde_json::Error> {
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

    fn fail<T>(&self, message: &str) -> Result<T, serde_json::Error> {
        Err(serde_json::Error::custom(format!(
            "{message} at byte {}",
            self.cursor
        )))
    }
}

const fn is_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
}

const fn is_value_delimiter(byte: u8) -> bool {
    is_whitespace(byte) || matches!(byte, b',' | b']' | b'}')
}
