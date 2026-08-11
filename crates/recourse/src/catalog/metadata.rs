//! Shared bounds for caller-visible catalog metadata.

/// Maximum number of characters in a diagnostic title.
pub const MAX_TITLE_CHARS: usize = 256;
/// Maximum number of characters in the default detail.
pub const MAX_DETAIL_CHARS: usize = 1_024;
/// Maximum number of caller suggestions on one diagnostic.
pub const MAX_SUGGESTIONS: usize = 32;
/// Maximum number of characters in one caller suggestion.
pub const MAX_SUGGESTION_CHARS: usize = 1_024;
/// Maximum number of characters in one diagnostic's Markdown documentation.
pub const MAX_DOCUMENTATION_CHARS: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataViolation {
    pub(crate) field: &'static str,
    pub(crate) reason: String,
}

pub(crate) fn validate(
    title: &str,
    detail: &str,
    suggestions: &[impl AsRef<str>],
    documentation: &str,
) -> Vec<MetadataViolation> {
    let mut violations = Vec::new();
    validate_text(&mut violations, "title", title, MAX_TITLE_CHARS, false);
    validate_text(&mut violations, "detail", detail, MAX_DETAIL_CHARS, false);
    if suggestions.len() > MAX_SUGGESTIONS {
        violations.push(MetadataViolation {
            field: "suggestions",
            reason: format!(
                "contains {} entries; maximum is {MAX_SUGGESTIONS}",
                suggestions.len()
            ),
        });
    }
    for suggestion in suggestions {
        validate_text(
            &mut violations,
            "suggestions",
            suggestion.as_ref(),
            MAX_SUGGESTION_CHARS,
            false,
        );
    }
    validate_text(
        &mut violations,
        "documentation",
        documentation,
        MAX_DOCUMENTATION_CHARS,
        true,
    );
    violations
}

fn validate_text(
    violations: &mut Vec<MetadataViolation>,
    field: &'static str,
    value: &str,
    maximum: usize,
    markdown: bool,
) {
    if value.trim().is_empty() {
        violations.push(violation(field, "must not be empty"));
        return;
    }
    let actual = value.chars().count();
    if actual > maximum {
        violations.push(violation(
            field,
            &format!("contains {actual} characters; maximum is {maximum}"),
        ));
    }
    if let Some(index) = value
        .chars()
        .position(|character| rejected_control(character, markdown))
    {
        violations.push(violation(
            field,
            &format!("contains a control character at index {index}"),
        ));
    }
}

fn rejected_control(character: char, markdown: bool) -> bool {
    character.is_control() && !(markdown && matches!(character, '\n' | '\r' | '\t'))
}

fn violation(field: &'static str, reason: &str) -> MetadataViolation {
    MetadataViolation {
        field,
        reason: reason.to_owned(),
    }
}
