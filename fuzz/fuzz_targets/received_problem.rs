//! Fuzzes bounded tolerant Problem decoding for determinism and preservation.
#![no_main]

use http::{HeaderMap, StatusCode};
use libfuzzer_sys::fuzz_target;
use recourse::client::{DecodeError, DecodeLimits, ReceivedProblem};

fuzz_target!(|body: &[u8]| {
    let first = decode(body);
    let second = decode(body);
    match (first, second) {
        (Ok(first), Ok(second)) => {
            assert_eq!(first.raw(), second.raw());
            assert_eq!(first.protocol_issues(), second.protocol_issues());
        }
        (Err(first), Err(second)) => assert_eq!(error_class(&first), error_class(&second)),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => panic!("identical input decoded inconsistently"),
    }
});

fn decode(body: &[u8]) -> Result<ReceivedProblem, DecodeError> {
    ReceivedProblem::from_slice(
        StatusCode::BAD_GATEWAY,
        &HeaderMap::new(),
        body,
        DecodeLimits::default(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorClass {
    MalformedJson,
    RootNotObject,
    LimitExceeded,
    Other,
}

fn error_class(error: &DecodeError) -> ErrorClass {
    match error {
        DecodeError::MalformedJson(_) => ErrorClass::MalformedJson,
        DecodeError::RootNotObject => ErrorClass::RootNotObject,
        DecodeError::LimitExceeded { .. } => ErrorClass::LimitExceeded,
        _ => ErrorClass::Other,
    }
}
