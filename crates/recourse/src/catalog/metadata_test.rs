//! Focused tests for static public metadata bounds.

use super::{
    MAX_DETAIL_CHARS, MAX_DOCUMENTATION_CHARS, MAX_SUGGESTION_CHARS, MAX_SUGGESTIONS,
    MAX_TITLE_CHARS, metadata,
};

#[test]
fn multibyte_metadata_is_bounded_by_characters() {
    let suggestions = vec!["é".repeat(MAX_SUGGESTION_CHARS); MAX_SUGGESTIONS];
    let violations = metadata::validate(
        &"é".repeat(MAX_TITLE_CHARS),
        &"é".repeat(MAX_DETAIL_CHARS),
        &suggestions,
        &"é".repeat(MAX_DOCUMENTATION_CHARS),
    );

    assert!(violations.is_empty());
}

#[test]
fn every_public_metadata_surface_has_a_bound_and_control_policy() {
    for (field, violations) in [
        (
            "title",
            metadata::validate(
                &"x".repeat(MAX_TITLE_CHARS + 1),
                "detail",
                &[] as &[&str],
                "docs",
            ),
        ),
        (
            "detail",
            metadata::validate(
                "title",
                &"x".repeat(MAX_DETAIL_CHARS + 1),
                &[] as &[&str],
                "docs",
            ),
        ),
        (
            "suggestions",
            metadata::validate(
                "title",
                "detail",
                &["x".repeat(MAX_SUGGESTION_CHARS + 1)],
                "docs",
            ),
        ),
        (
            "documentation",
            metadata::validate(
                "title",
                "detail",
                &[] as &[&str],
                &"x".repeat(MAX_DOCUMENTATION_CHARS + 1),
            ),
        ),
    ] {
        assert!(
            violations.iter().any(|violation| violation.field == field),
            "missing {field} violation"
        );
    }

    let too_many = vec!["help"; MAX_SUGGESTIONS + 1];
    assert!(!metadata::validate("title", "detail", &too_many, "docs").is_empty());
    assert!(!metadata::validate("ti\u{7}tle", "detail", &[] as &[&str], "docs").is_empty());
    assert!(!metadata::validate("title", "detail", &["bad\u{7}"], "docs").is_empty());
    assert!(!metadata::validate("title", "detail", &[] as &[&str], "bad\u{7}").is_empty());
    assert!(metadata::validate("title", "detail", &[] as &[&str], "one\n\ntwo").is_empty());
}
