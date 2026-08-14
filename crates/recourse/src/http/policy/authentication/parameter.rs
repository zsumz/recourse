//! Authentication parameter parsing and case-insensitive uniqueness checks.

use super::grammar::{is_quoted_pair_byte, is_quoted_text, parse_token, skip_ows};

pub(super) fn parse_parameters(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let (_, mut cursor) = parse_parameter(bytes, start)?;
    loop {
        let end = cursor;
        cursor = skip_ows(bytes, cursor);
        if cursor == bytes.len() {
            return Some((end, cursor));
        }
        if bytes[cursor] != b',' {
            return None;
        }
        let candidate = skip_ows(bytes, cursor + 1);
        let Some((_, next)) = parse_parameter(bytes, candidate) else {
            if starts_parameter(bytes, candidate) {
                return None;
            }
            return Some((end, cursor));
        };
        cursor = next;
    }
}

pub(super) fn parameters_unique(parameters: &[u8]) -> bool {
    let mut names = Vec::<&[u8]>::new();
    visit_parameters(parameters, |parameter| {
        if names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(parameter.name))
        {
            return false;
        }
        names.push(parameter.name);
        true
    })
}

pub(super) fn visit_parameters<'a>(
    parameters: &'a [u8],
    mut visit: impl FnMut(Parameter<'a>) -> bool,
) -> bool {
    let mut cursor = 0;
    while cursor < parameters.len() {
        let Some((parameter, next)) = parse_parameter(parameters, cursor) else {
            return false;
        };
        if !visit(parameter) {
            return false;
        }
        cursor = skip_ows(parameters, next);
        if cursor == parameters.len() {
            return true;
        }
        if parameters[cursor] != b',' {
            return false;
        }
        cursor = skip_ows(parameters, cursor + 1);
    }
    false
}

#[derive(Clone, Copy)]
pub(super) struct Parameter<'a> {
    pub(super) name: &'a [u8],
    pub(super) value: &'a [u8],
    pub(super) quoted: bool,
}

impl Parameter<'_> {
    pub(super) fn value_eq(self, expected: &[u8]) -> bool {
        let mut actual = self.value.iter().copied();
        expected.iter().copied().all(|wanted| {
            let Some(mut received) = actual.next() else {
                return false;
            };
            if self.quoted && received == b'\\' {
                let Some(escaped) = actual.next() else {
                    return false;
                };
                received = escaped;
            }
            received.eq_ignore_ascii_case(&wanted)
        }) && actual.next().is_none()
    }
}

fn starts_parameter(bytes: &[u8], start: usize) -> bool {
    parse_token(bytes, start)
        .map(|name_end| skip_ows(bytes, name_end))
        .is_some_and(|equals| bytes.get(equals) == Some(&b'='))
}

fn parse_parameter(bytes: &[u8], start: usize) -> Option<(Parameter<'_>, usize)> {
    let name_end = parse_token(bytes, start)?;
    let equals = skip_ows(bytes, name_end);
    if bytes.get(equals) != Some(&b'=') {
        return None;
    }
    let value_start = skip_ows(bytes, equals + 1);
    if bytes.get(value_start) == Some(&b'"') {
        let (value_end, next) = parse_quoted(bytes, value_start)?;
        return Some((
            Parameter {
                name: &bytes[start..name_end],
                value: &bytes[value_start + 1..value_end],
                quoted: true,
            },
            next,
        ));
    }
    let value_end = parse_token(bytes, value_start)?;
    Some((
        Parameter {
            name: &bytes[start..name_end],
            value: &bytes[value_start..value_end],
            quoted: false,
        },
        value_end,
    ))
}

fn parse_quoted(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => return Some((cursor, cursor + 1)),
            b'\\' => {
                cursor += 1;
                if cursor == bytes.len() || !is_quoted_pair_byte(bytes[cursor]) {
                    return None;
                }
            }
            byte if !is_quoted_text(byte) => return None,
            _ => {}
        }
        cursor += 1;
    }
    None
}
