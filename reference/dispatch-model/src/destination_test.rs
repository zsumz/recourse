//! Destination validation and serialization tests.

use super::{Destination, DestinationError};

#[test]
fn destination_accepts_bounded_public_text() {
    let destination = Destination::new("warehouse-west")
        .unwrap_or_else(|error| panic!("test destination must be valid: {error}"));

    assert_eq!(destination.as_str(), "warehouse-west");
    assert_eq!(
        serde_json::to_string(&destination).ok().as_deref(),
        Some("\"warehouse-west\"")
    );
}

#[test]
fn destination_rejects_empty_control_and_overlong_values() {
    assert_eq!(Destination::new(""), Err(DestinationError::Empty));
    assert!(matches!(
        Destination::new("unsafe\nvalue"),
        Err(DestinationError::ControlCharacter { .. })
    ));
    assert!(matches!(
        Destination::new("x".repeat(257)),
        Err(DestinationError::TooLong { actual_bytes: 257 })
    ));
}
