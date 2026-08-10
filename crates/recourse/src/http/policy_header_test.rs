//! Exact standard-header fixtures for built-in HTTP policies.

use std::time::{Duration, UNIX_EPOCH};

use http::{
    Method,
    header::{ALLOW, RETRY_AFTER, WWW_AUTHENTICATE},
};

use super::{
    AllowedMethods, AllowedMethodsError, BearerChallenge, BearerChallengeError, HttpPolicy,
    MethodNotAllowed, RetryAfter, RetryAfterPolicy, Unauthorized,
};

#[test]
fn unauthorized_emits_one_escaped_bearer_challenge() {
    let challenge = BearerChallenge::new("dispatch \"jobs\"");
    let Some(challenge) = challenge.ok() else {
        return;
    };
    let headers = Unauthorized::headers(challenge);

    assert_eq!(
        headers
            .ok()
            .and_then(|value| value.get(WWW_AUTHENTICATE).cloned()),
        Some(http::HeaderValue::from_static(
            "Bearer realm=\"dispatch \\\"jobs\\\"\""
        ))
    );
    assert_eq!(
        BearerChallenge::new(""),
        Err(BearerChallengeError::EmptyRealm)
    );
    assert!(matches!(
        BearerChallenge::new("dispatch\n"),
        Err(BearerChallengeError::InvalidByte { .. })
    ));
}

#[test]
fn method_not_allowed_sorts_and_deduplicates_allow_values() {
    let methods = AllowedMethods::new([Method::POST, Method::GET, Method::POST]);
    let Some(methods) = methods.ok() else {
        return;
    };
    let headers = MethodNotAllowed::headers(methods);

    assert_eq!(
        headers.ok().and_then(|value| value.get(ALLOW).cloned()),
        Some(http::HeaderValue::from_static("GET, POST"))
    );
    assert_eq!(
        AllowedMethods::new(Vec::<Method>::new()),
        Err(AllowedMethodsError::Empty)
    );
}

#[test]
fn allowed_methods_equality_ignores_order_and_repetition() {
    let declared = AllowedMethods::new([Method::GET, Method::POST]);
    let reversed = AllowedMethods::new([Method::POST, Method::GET, Method::POST]);
    let literal = AllowedMethods::from_static(&[Method::POST, Method::GET]);
    let narrower = AllowedMethods::new([Method::GET]);

    assert_eq!(declared, reversed);
    assert_eq!(declared.as_ref().ok(), Some(&literal));
    assert_ne!(declared, narrower);
    assert_eq!(
        AllowedMethods::new([Method::GET, Method::GET]),
        AllowedMethods::new([Method::GET])
    );
}

#[test]
fn retry_after_rounds_delays_up_to_seconds() {
    let delayed = RetryAfterPolicy::<503>::headers(RetryAfter::after(Duration::from_millis(1)));
    let exact = RetryAfterPolicy::<429>::headers(RetryAfter::after(Duration::from_secs(30)));

    assert_eq!(
        delayed
            .ok()
            .and_then(|value| value.get(RETRY_AFTER).cloned()),
        Some(http::HeaderValue::from_static("1"))
    );
    assert_eq!(
        exact.ok().and_then(|value| value.get(RETRY_AFTER).cloned()),
        Some(http::HeaderValue::from_static("30"))
    );
}

#[test]
fn retry_after_formats_imf_fixdate() {
    let time = UNIX_EPOCH + Duration::from_secs(784_111_777);
    let headers = RetryAfterPolicy::<503>::headers(RetryAfter::at(time));

    assert_eq!(
        headers
            .ok()
            .and_then(|value| value.get(RETRY_AFTER).cloned()),
        Some(http::HeaderValue::from_static(
            "Sun, 06 Nov 1994 08:49:37 GMT"
        ))
    );
}
