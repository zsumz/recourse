//! Tests for replaceable and canonical request-ID generation.

use recourse::http::MAX_CORRELATION_ID_BYTES;

use super::{RequestIdGenerator, UlidRequestIds};

#[test]
fn ulid_generator_returns_distinct_canonical_values() {
    let generator = UlidRequestIds;
    let first = generator.generate().unwrap_or_else(|error| {
        panic!("ULID generator must satisfy the correlation contract: {error}")
    });
    let second = generator.generate().unwrap_or_else(|error| {
        panic!("ULID generator must satisfy the correlation contract: {error}")
    });

    assert_eq!(first.as_str().len(), 26);
    assert!(first.as_str().len() <= MAX_CORRELATION_ID_BYTES);
    assert_ne!(first, second);
    assert!(first.as_str().bytes().all(|byte| byte.is_ascii_graphic()));
}

#[test]
fn applications_can_supply_a_deterministic_generator() {
    struct Deterministic;

    impl RequestIdGenerator for Deterministic {
        fn generate(
            &self,
        ) -> Result<recourse::http::CorrelationId, recourse::http::CorrelationIdError> {
            recourse::http::CorrelationId::new("test-request")
        }
    }

    let generated = Deterministic.generate().unwrap_or_else(|error| {
        panic!("test request ID must satisfy the public contract: {error}")
    });
    assert_eq!(generated.as_str(), "test-request");
}
