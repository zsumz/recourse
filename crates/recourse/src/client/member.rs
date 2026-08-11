//! Shared tolerant extraction of diagnostic identity and display members.

use std::str::FromStr;

use serde_json::{Map, Value};

use crate::catalog::Code;

use super::ProtocolIssue;

pub(super) fn string(
    raw: &Map<String, Value>,
    name: &'static str,
    issues: &mut Vec<ProtocolIssue>,
) -> Option<String> {
    match raw.get(name) {
        None => None,
        Some(Value::String(value)) => Some(value.clone()),
        Some(_) => {
            invalid_type(issues, name, "string");
            None
        }
    }
}

pub(super) fn object(
    raw: &Map<String, Value>,
    name: &'static str,
    issues: &mut Vec<ProtocolIssue>,
) -> Option<Map<String, Value>> {
    match raw.get(name) {
        None => None,
        Some(Value::Object(value)) => Some(value.clone()),
        Some(_) => {
            invalid_type(issues, name, "object");
            None
        }
    }
}

pub(super) fn string_array(
    raw: &Map<String, Value>,
    name: &'static str,
    issues: &mut Vec<ProtocolIssue>,
) -> Vec<String> {
    let Some(value) = raw.get(name) else {
        return Vec::new();
    };
    let Value::Array(values) = value else {
        invalid_type(issues, name, "array of strings");
        return Vec::new();
    };
    let strings = values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if strings.len() != values.len() {
        invalid_type(issues, name, "array of strings");
    }
    strings
}

pub(super) fn code(raw: &Map<String, Value>, issues: &mut Vec<ProtocolIssue>) -> Option<Code> {
    let value = string(raw, "code", issues)?;
    if let Ok(code) = Code::from_str(&value) {
        Some(code)
    } else {
        issues.push(ProtocolIssue::MalformedCode);
        None
    }
}

pub(super) fn invalid_type(
    issues: &mut Vec<ProtocolIssue>,
    member: &'static str,
    expected: &'static str,
) {
    issues.push(ProtocolIssue::InvalidMemberType { member, expected });
}
