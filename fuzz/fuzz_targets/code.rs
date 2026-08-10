//! Fuzzes canonical diagnostic-code parsing and formatting.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::catalog::Code;

fuzz_target!(|input: &[u8]| {
    let Ok(text) = std::str::from_utf8(input) else {
        return;
    };
    let text = text.trim_end_matches(['\r', '\n']);
    let first = text.parse::<Code>();
    let second = text.parse::<Code>();
    assert_eq!(first, second);
    if let Ok(code) = first {
        assert_eq!(code.to_string().parse::<Code>(), Ok(code));
    }
});
