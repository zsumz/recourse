//! Conservative recursive compatibility for the normalized schema profile.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::catalog::{Code, schema::number};

use super::{ChangeInput, CompatibilityChange, push};

pub(super) fn compare(
    code: &Code,
    path: &str,
    previous: &Value,
    current: &Value,
    changes: &mut Vec<CompatibilityChange>,
) {
    if number::values_equal(previous, current) {
        return;
    }
    let (Some(previous), Some(current)) = (previous.as_object(), current.as_object()) else {
        push(changes, ChangeInput::schema_changed(code, path));
        return;
    };
    compare_properties(code, path, previous, current, changes);
    compare_requiredness(code, path, previous, current, changes);
    compare_definitions(code, path, previous, current, changes);
    compare_items(code, path, previous, current, changes);
    compare_other_keywords(code, path, previous, current, changes);
}

fn compare_properties(
    code: &Code,
    path: &str,
    previous: &Map<String, Value>,
    current: &Map<String, Value>,
    changes: &mut Vec<CompatibilityChange>,
) {
    let empty = Map::new();
    let old = object_keyword(previous, "properties").unwrap_or(&empty);
    let new = object_keyword(current, "properties").unwrap_or(&empty);
    let required = required_fields(current);
    let accepted_by_previous = accepts_unknown_properties(previous);
    for (name, schema) in old {
        let field_path = format!("{path}.properties.{name}");
        match new.get(name) {
            Some(value) => compare(code, &field_path, schema, value, changes),
            None => push(changes, ChangeInput::property_removed(code, &field_path)),
        }
    }
    for name in new.keys().filter(|name| !old.contains_key(*name)) {
        let field_path = format!("{path}.properties.{name}");
        let change = if required.contains(name.as_str()) {
            ChangeInput::required_property(code, &field_path)
        } else if accepted_by_previous {
            ChangeInput::optional_property(code, &field_path)
        } else {
            ChangeInput::rejected_optional_property(code, &field_path)
        };
        push(changes, change);
    }
}

fn accepts_unknown_properties(object: &Map<String, Value>) -> bool {
    object
        .get("additionalProperties")
        .is_none_or(|value| value == &Value::Bool(true))
}

fn compare_requiredness(
    code: &Code,
    path: &str,
    previous: &Map<String, Value>,
    current: &Map<String, Value>,
    changes: &mut Vec<CompatibilityChange>,
) {
    let old = required_fields(previous);
    let new = required_fields(current);
    let names = old.union(&new).copied().collect::<BTreeSet<_>>();
    let empty = Map::new();
    let old_properties = object_keyword(previous, "properties").unwrap_or(&empty);
    let new_properties = object_keyword(current, "properties").unwrap_or(&empty);
    for name in names {
        if !old_properties.contains_key(name) || !new_properties.contains_key(name) {
            continue;
        }
        let was_required = old.contains(name);
        let is_required = new.contains(name);
        if was_required != is_required {
            push(
                changes,
                ChangeInput::requiredness(
                    code,
                    &format!("{path}.properties.{name}"),
                    was_required,
                    is_required,
                ),
            );
        }
    }
}

fn compare_definitions(
    code: &Code,
    path: &str,
    previous: &Map<String, Value>,
    current: &Map<String, Value>,
    changes: &mut Vec<CompatibilityChange>,
) {
    let empty = Map::new();
    let old = object_keyword(previous, "$defs").unwrap_or(&empty);
    let new = object_keyword(current, "$defs").unwrap_or(&empty);
    for (name, schema) in old {
        let definition_path = format!("{path}.$defs.{name}");
        match new.get(name) {
            Some(value) => compare(code, &definition_path, schema, value, changes),
            None => push(changes, ChangeInput::schema_changed(code, &definition_path)),
        }
    }
}

fn compare_items(
    code: &Code,
    path: &str,
    previous: &Map<String, Value>,
    current: &Map<String, Value>,
    changes: &mut Vec<CompatibilityChange>,
) {
    match (previous.get("items"), current.get("items")) {
        (Some(old), Some(new)) => compare(code, &format!("{path}.items"), old, new, changes),
        (None, None) => {}
        (Some(_), None) | (None, Some(_)) => push(
            changes,
            ChangeInput::schema_changed(code, &format!("{path}.items")),
        ),
    }
}

fn compare_other_keywords(
    code: &Code,
    path: &str,
    previous: &Map<String, Value>,
    current: &Map<String, Value>,
    changes: &mut Vec<CompatibilityChange>,
) {
    let ignored = ["properties", "required", "$defs", "items"];
    let keys = previous
        .keys()
        .chain(current.keys())
        .filter(|key| !ignored.contains(&key.as_str()))
        .collect::<BTreeSet<_>>();
    for key in keys {
        if !matches!((previous.get(key), current.get(key)),
            (Some(previous), Some(current)) if number::values_equal(previous, current))
        {
            push(
                changes,
                ChangeInput::schema_changed(code, &format!("{path}.{key}")),
            );
        }
    }
}

fn object_keyword<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    object.get(key).and_then(Value::as_object)
}

fn required_fields(object: &Map<String, Value>) -> BTreeSet<&str> {
    object
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}
