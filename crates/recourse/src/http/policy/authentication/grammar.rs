//! Byte-level HTTP field grammar shared by authentication parsing.

pub(super) fn parse_token(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start;
    while bytes.get(cursor).is_some_and(|byte| is_token_byte(*byte)) {
        cursor += 1;
    }
    (cursor > start).then_some(cursor)
}

pub(super) fn skip_list_delimiters(bytes: &[u8], mut cursor: usize) -> usize {
    loop {
        cursor = skip_ows(bytes, cursor);
        if bytes.get(cursor) != Some(&b',') {
            return cursor;
        }
        cursor += 1;
    }
}

pub(super) fn skip_ows(bytes: &[u8], mut cursor: usize) -> usize {
    while bytes.get(cursor).is_some_and(|byte| is_ows(*byte)) {
        cursor += 1;
    }
    cursor
}

pub(super) const fn is_ows(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

const fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'..=b'\'' | b'*' | b'+' | b'-' | b'.' | b'^'..=b'`' | b'|' | b'~'
        )
}

pub(super) const fn is_token68_base(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/')
}

pub(super) const fn is_quoted_text(byte: u8) -> bool {
    matches!(
        byte,
        b'\t' | b' ' | b'!' | b'#'..=b'[' | b']'..=b'~' | 0x80..=0xff
    )
}

pub(super) const fn is_quoted_pair_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' '..=b'~' | 0x80..=0xff)
}
