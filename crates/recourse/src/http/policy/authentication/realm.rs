//! Shared bounded realm validation and quoted-string encoding.

pub(super) const MAX_REALM_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RealmIssue {
    Empty,
    TooLong { actual_bytes: usize },
    InvalidByte { byte_index: usize, byte: u8 },
}

pub(super) fn validate(realm: &str) -> Result<(), RealmIssue> {
    if realm.is_empty() {
        return Err(RealmIssue::Empty);
    }
    if realm.len() > MAX_REALM_BYTES {
        return Err(RealmIssue::TooLong {
            actual_bytes: realm.len(),
        });
    }
    if let Some((byte_index, byte)) = realm
        .bytes()
        .enumerate()
        .find(|(_, byte)| !is_visible_ascii(*byte))
    {
        return Err(RealmIssue::InvalidByte { byte_index, byte });
    }
    Ok(())
}

pub(super) fn escape(realm: &str) -> String {
    realm.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) const fn is_visible_ascii(byte: u8) -> bool {
    matches!(byte, b' '..=b'~')
}
