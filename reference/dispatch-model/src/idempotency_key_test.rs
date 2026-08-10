//! Idempotency header identity tests.

use super::{IdempotencyKey, IdempotencyKeyError};

#[test]
fn idempotency_key_is_bounded_visible_ascii() {
    let key = IdempotencyKey::new("create-42")
        .unwrap_or_else(|error| panic!("test key must be valid: {error}"));

    assert_eq!(key.as_str(), "create-42");
    assert_eq!(IdempotencyKey::new(""), Err(IdempotencyKeyError::Empty));
    assert!(matches!(
        IdempotencyKey::new("contains space"),
        Err(IdempotencyKeyError::InvalidByte { .. })
    ));
    assert!(matches!(
        IdempotencyKey::new("x".repeat(129)),
        Err(IdempotencyKeyError::TooLong { actual_bytes: 129 })
    ));
}
