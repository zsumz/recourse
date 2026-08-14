//! Structural parsing for one or more HTTP authentication challenges.

use super::grammar::{parse_token, skip_list_delimiters, skip_ows};

#[derive(Clone, Copy)]
pub(super) struct Challenge<'a> {
    pub(super) scheme: &'a [u8],
    pub(super) data: ChallengeData<'a>,
}

#[derive(Clone, Copy)]
pub(super) enum ChallengeData<'a> {
    None,
    Token68,
    Parameters(&'a [u8]),
}

pub(super) fn field_has_challenge(bytes: &[u8], accepts: &impl Fn(Challenge<'_>) -> bool) -> bool {
    let mut cursor = 0;
    let mut accepted = false;
    loop {
        cursor = skip_list_delimiters(bytes, cursor);
        if cursor == bytes.len() {
            return accepted;
        }
        let Some((challenge, next)) = parse_challenge(bytes, cursor) else {
            return accepted;
        };
        accepted |= accepts(challenge);
        cursor = next;
    }
}

impl Challenge<'_> {
    pub(super) fn valid_basic_parameters(self) -> bool {
        let ChallengeData::Parameters(parameters) = self.data else {
            return false;
        };
        let mut cursor = 0;
        let mut saw_realm = false;
        let mut saw_charset = false;
        while cursor < parameters.len() {
            let Some((parameter, next)) = parse_parameter(parameters, cursor) else {
                return false;
            };
            if parameter.name.eq_ignore_ascii_case(b"realm") {
                if saw_realm || !parameter.quoted || parameter.value.is_empty() {
                    return false;
                }
                saw_realm = true;
            } else if parameter.name.eq_ignore_ascii_case(b"charset") {
                if saw_charset || !parameter.value_eq(b"UTF-8") {
                    return false;
                }
                saw_charset = true;
            }
            cursor = skip_ows(parameters, next);
            if cursor == parameters.len() {
                break;
            }
            if parameters[cursor] != b',' {
                return false;
            }
            cursor = skip_ows(parameters, cursor + 1);
        }
        saw_realm
    }
}

fn parse_challenge(bytes: &[u8], start: usize) -> Option<(Challenge<'_>, usize)> {
    let scheme_end = parse_token(bytes, start)?;
    let scheme = &bytes[start..scheme_end];
    if scheme_end == bytes.len() || bytes[scheme_end] == b',' {
        return Some((
            Challenge {
                scheme,
                data: ChallengeData::None,
            },
            scheme_end,
        ));
    }
    if !super::grammar::is_ows(bytes[scheme_end]) {
        return None;
    }
    let data_start = skip_ows(bytes, scheme_end);
    if data_start == bytes.len() || bytes[data_start] == b',' {
        return Some((
            Challenge {
                scheme,
                data: ChallengeData::None,
            },
            data_start,
        ));
    }
    if let Some((data_end, next)) = parse_parameters(bytes, data_start) {
        return Some((
            Challenge {
                scheme,
                data: ChallengeData::Parameters(&bytes[data_start..data_end]),
            },
            next,
        ));
    }
    let next = parse_token68(bytes, data_start)?;
    Some((
        Challenge {
            scheme,
            data: ChallengeData::Token68,
        },
        next,
    ))
}

fn parse_parameters(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
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

fn starts_parameter(bytes: &[u8], start: usize) -> bool {
    parse_token(bytes, start)
        .map(|name_end| skip_ows(bytes, name_end))
        .is_some_and(|equals| bytes.get(equals) == Some(&b'='))
}

#[derive(Clone, Copy)]
struct Parameter<'a> {
    name: &'a [u8],
    value: &'a [u8],
    quoted: bool,
}

impl Parameter<'_> {
    fn value_eq(self, expected: &[u8]) -> bool {
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
                if cursor == bytes.len() || !super::grammar::is_quoted_pair_byte(bytes[cursor]) {
                    return None;
                }
            }
            byte if !super::grammar::is_quoted_text(byte) => return None,
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn parse_token68(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    while bytes
        .get(cursor)
        .is_some_and(|byte| super::grammar::is_token68_base(*byte))
    {
        cursor += 1;
    }
    if cursor == start {
        return None;
    }
    while bytes.get(cursor) == Some(&b'=') {
        cursor += 1;
    }
    cursor = skip_ows(bytes, cursor);
    (cursor == bytes.len() || bytes[cursor] == b',').then_some(cursor)
}
