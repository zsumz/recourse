//! Exact standard-header fixtures for built-in HTTP policies.

use std::time::{Duration, UNIX_EPOCH};

use http::{
    Method,
    header::{ALLOW, RETRY_AFTER},
};

use super::{
    AllowedMethods, AllowedMethodsError, HttpPolicy, MethodNotAllowed, RetryAfter, RetryAfterError,
    RetryAfterPolicy,
};

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
    let Ok(retry_after) = RetryAfter::try_at(time) else {
        panic!("fixture date must be representable");
    };
    let headers = RetryAfterPolicy::<503>::headers(retry_after);

    assert_eq!(
        headers
            .ok()
            .and_then(|value| value.get(RETRY_AFTER).cloned()),
        Some(http::HeaderValue::from_static(
            "Sun, 06 Nov 1994 08:49:37 GMT"
        ))
    );
}

#[test]
fn retry_after_rejects_both_http_date_boundaries_without_panicking() {
    let Some(before_epoch) = UNIX_EPOCH.checked_sub(Duration::from_secs(1)) else {
        panic!("platform must represent the lower boundary");
    };
    let Some(after_range) = UNIX_EPOCH.checked_add(Duration::from_hours(70_389_528)) else {
        panic!("platform must represent the upper boundary");
    };
    let Some(last_supported) = after_range.checked_sub(Duration::from_secs(1)) else {
        panic!("platform must represent the last supported second");
    };

    assert!(RetryAfter::try_at(UNIX_EPOCH).is_ok());
    assert!(RetryAfter::try_at(last_supported).is_ok());
    assert_eq!(
        RetryAfter::try_at(before_epoch),
        Err(RetryAfterError::BeforeUnixEpoch)
    );
    assert_eq!(
        RetryAfter::try_at(after_range),
        Err(RetryAfterError::AfterHttpDateRange)
    );
    for time in [before_epoch, UNIX_EPOCH, last_supported, after_range] {
        assert!(std::panic::catch_unwind(|| RetryAfter::try_at(time)).is_ok());
    }
}
