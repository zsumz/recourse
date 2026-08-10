//! Focused tests for the adapter-neutral encoded response boundary.

use http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE};

use super::EncodedProblem;

#[test]
fn encoded_problem_splits_without_framework_types() {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    let encoded = EncodedProblem::new(StatusCode::NOT_FOUND, headers.clone(), b"{}".to_vec());

    assert_eq!(encoded.status(), StatusCode::NOT_FOUND);
    assert_eq!(encoded.headers(), &headers);
    assert_eq!(encoded.body(), b"{}");
    assert_eq!(
        encoded.into_parts(),
        (StatusCode::NOT_FOUND, headers, b"{}".to_vec())
    );
}

#[test]
fn splitting_transfers_the_encoded_body_without_reallocation() {
    let body = br#"{"code":"PERF-1"}"#.to_vec();
    let allocation = body.as_ptr();
    let encoded = EncodedProblem::new(StatusCode::BAD_REQUEST, HeaderMap::new(), body);

    let (_, _, transferred) = encoded.into_parts();

    assert_eq!(transferred.as_ptr(), allocation);
}
