//! Focused tests for sealed fixed HTTP policy metadata.

use super::{Fixed, HttpPolicy};

#[test]
fn fixed_policy_has_one_status_and_no_required_headers() {
    type NotFound = Fixed<404>;

    assert_eq!(NotFound::STATUS, 404);
    assert_eq!(NotFound::NAME, "fixed");
    assert!(NotFound::REQUIRED_HEADERS.is_empty());
    assert!(NotFound::headers(()).is_ok_and(|headers| headers.is_empty()));
}
