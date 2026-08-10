//! Terminal escaping tests for controls, hyperlinks, and bidi spoofing.

use super::escape_terminal;

#[test]
fn ordinary_unicode_text_is_preserved() {
    assert_eq!(
        escape_terminal("Dispatch failed: café 東京"),
        "Dispatch failed: café 東京"
    );
}

#[test]
fn c0_c1_and_delete_controls_are_visible() {
    assert_eq!(
        escape_terminal("nul\0 tab\t line\n esc\u{1b} c1\u{009b} del\u{007f}"),
        "nul\\u{0} tab\\t line\\n esc\\u{1b} c1\\u{9b} del\\u{7f}"
    );
}

#[test]
fn ansi_and_osc_hyperlink_sequences_cannot_execute() {
    let attack =
        "\u{1b}[31mred\u{1b}[0m \u{1b}]8;;https://evil.invalid\u{1b}\\click\u{1b}]8;;\u{1b}\\";
    let escaped = escape_terminal(attack);

    assert!(!escaped.contains('\u{1b}'));
    assert!(escaped.contains("\\u{1b}[31mred"));
    assert!(escaped.contains("\\u{1b}]8;;https://evil.invalid"));
}

#[test]
fn every_unicode_bidi_control_is_visible() {
    let controls = "\u{061c}\u{200e}\u{200f}\u{202a}\u{202b}\u{202c}\u{202d}\u{202e}\u{2066}\u{2067}\u{2068}\u{2069}";
    let escaped = escape_terminal(controls);

    assert_eq!(
        escaped,
        "\\u{61c}\\u{200e}\\u{200f}\\u{202a}\\u{202b}\\u{202c}\\u{202d}\\u{202e}\\u{2066}\\u{2067}\\u{2068}\\u{2069}"
    );
}
