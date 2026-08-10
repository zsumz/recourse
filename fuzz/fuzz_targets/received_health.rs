//! Fuzzes bounded health-finding decoding for deterministic preservation.
#![no_main]

use libfuzzer_sys::fuzz_target;
use recourse::client::{DecodeError, DecodeLimit, DecodeLimits, ReceivedHealthFinding};

fuzz_target!(|body: &[u8]| {
    let first = ReceivedHealthFinding::from_slice(body, DecodeLimits::default());
    let second = ReceivedHealthFinding::from_slice(body, DecodeLimits::default());
    match (first, second) {
        (Ok(first), Ok(second)) => {
            assert_eq!(first.raw(), second.raw());
            assert_eq!(first.protocol_issues(), second.protocol_issues());
        }
        (Err(first), Err(second)) => assert_eq!(error_class(&first), error_class(&second)),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("identical input decoded inconsistently"),
    }
});

fn error_class(error: &DecodeError) -> (u8, Option<DecodeLimit>) {
    match error {
        DecodeError::MalformedJson(_) => (0, None),
        DecodeError::RootNotObject => (1, None),
        DecodeError::LimitExceeded { limit, .. } => (2, Some(*limit)),
    }
}
