//! Structural parsing for one or more HTTP authentication challenges.

use super::{
    grammar::{parse_token, skip_list_delimiters, skip_ows},
    parameter::{parameters_unique, parse_parameters, visit_parameters},
};

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
        if !parameters_unique(parameters) {
            return false;
        }
        let mut saw_realm = false;
        let valid = visit_parameters(parameters, |parameter| {
            if parameter.name.eq_ignore_ascii_case(b"realm") {
                if !parameter.quoted || parameter.value.is_empty() {
                    return false;
                }
                saw_realm = true;
            } else if parameter.name.eq_ignore_ascii_case(b"charset")
                && !parameter.value_eq(b"UTF-8")
            {
                return false;
            }
            true
        });
        valid && saw_realm
    }

    pub(super) fn has_unique_parameters(self) -> bool {
        let ChallengeData::Parameters(parameters) = self.data else {
            return false;
        };
        parameters_unique(parameters)
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
