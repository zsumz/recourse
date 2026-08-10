//! Terminal-safe rendering of untrusted protocol text.

/// Escapes control and bidirectional-formatting characters in untrusted text.
///
/// The returned text cannot activate ANSI controls or OSC hyperlinks because
/// their C0 or C1 introducers are rendered visibly. Ordinary Unicode text is
/// preserved.
pub fn escape_terminal(untrusted: &str) -> String {
    let mut escaped = String::with_capacity(untrusted.len());
    for character in untrusted.chars() {
        if character.is_control() || is_bidi_control(character) {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}
