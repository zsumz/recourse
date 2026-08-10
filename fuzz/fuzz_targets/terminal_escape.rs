//! Fuzzes terminal escaping for control removal and idempotence.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::client::escape_terminal;

fuzz_target!(|input: &[u8]| {
    let Ok(input) = std::str::from_utf8(input) else {
        return;
    };
    let escaped = escape_terminal(input);
    assert!(!escaped.chars().any(is_unsafe));
    assert_eq!(escape_terminal(&escaped), escaped);
});

fn is_unsafe(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200e}'
                | '\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2066}'..='\u{2069}'
        )
}
