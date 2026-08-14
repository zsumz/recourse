//! Policy-level recognition of received HTTP authentication challenges.

use http::{HeaderMap, header::WWW_AUTHENTICATE};

use super::challenge::{Challenge, ChallengeData, field_has_challenge};

pub(super) fn has_valid_basic_challenge(headers: &HeaderMap) -> bool {
    has_challenge(headers, |challenge| {
        challenge.scheme.eq_ignore_ascii_case(b"Basic") && challenge.valid_basic_parameters()
    })
}

pub(super) fn has_valid_bearer_challenge(headers: &HeaderMap) -> bool {
    has_challenge(headers, |challenge| {
        challenge.scheme.eq_ignore_ascii_case(b"Bearer")
            && matches!(challenge.data, ChallengeData::Parameters(_))
            && challenge.has_unique_parameters()
    })
}

fn has_challenge(headers: &HeaderMap, accepts: impl Fn(Challenge<'_>) -> bool) -> bool {
    headers
        .get_all(WWW_AUTHENTICATE)
        .iter()
        .any(|value| field_has_challenge(value.as_bytes(), &accepts))
}
