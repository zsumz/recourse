//! Shared tolerant extraction of diagnostic identity and display members.

use std::str::FromStr;

use serde_json::{Map, Value};

use crate::catalog::Code;

use super::ProtocolIssue;

pub(super) fn string(raw: &Map<String, Value>, name: &str) -> Option<String> {
    raw.get(name).and_then(Value::as_str).map(str::to_owned)
}

pub(super) fn object(raw: &Map<String, Value>, name: &str) -> Option<Map<String, Value>> {
    raw.get(name).and_then(Value::as_object).cloned()
}

pub(super) fn string_array(raw: &Map<String, Value>, name: &str) -> Vec<String> {
    raw.get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub(super) fn code(raw: &Map<String, Value>, issues: &mut Vec<ProtocolIssue>) -> Option<Code> {
    let value = raw.get("code")?.as_str()?;
    if let Ok(code) = Code::from_str(value) {
        Some(code)
    } else {
        issues.push(ProtocolIssue::MalformedCode);
        None
    }
}
