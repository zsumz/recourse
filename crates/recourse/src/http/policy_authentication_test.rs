//! Exact and hostile fixtures for built-in authentication challenges.

use http::{HeaderValue, StatusCode, header::WWW_AUTHENTICATE};

use super::{
    BasicChallenge, BasicChallengeError, BasicUnauthorized, BearerChallenge, BearerChallengeError,
    BearerUnauthorized, HttpPolicy,
};

fn authentication_header<P: HttpPolicy>(input: P::Input) -> Option<HeaderValue> {
    P::headers(input)
        .ok()
        .and_then(|headers| headers.get(WWW_AUTHENTICATE).cloned())
}

#[test]
fn basic_unauthorized_emits_the_ballast_registry_challenge() {
    let Some(challenge) = BasicChallenge::new("ballast-registry").ok() else {
        return;
    };
    let headers = BasicUnauthorized::headers(challenge);

    assert_eq!(BasicUnauthorized::STATUS, StatusCode::UNAUTHORIZED.as_u16());
    assert_eq!(BasicUnauthorized::NAME, "basic_unauthorized");
    assert_eq!(BasicUnauthorized::REQUIRED_HEADERS, &["www-authenticate"]);
    assert!(headers.is_ok_and(|headers| {
        headers.len() == 1
            && headers
                .get_all(WWW_AUTHENTICATE)
                .iter()
                .eq([&HeaderValue::from_static(
                    "Basic realm=\"ballast-registry\"",
                )])
    }));
}

#[test]
fn basic_challenge_escapes_only_quoted_string_delimiters() {
    let challenge = BasicChallenge::new("registry \\\"mirror\\\"");

    assert_eq!(
        challenge
            .ok()
            .and_then(authentication_header::<BasicUnauthorized>),
        Some(HeaderValue::from_static(
            "Basic realm=\"registry \\\\\\\"mirror\\\\\\\"\""
        ))
    );
}

#[test]
fn basic_challenge_can_advertise_the_only_defined_charset() {
    let challenge = BasicChallenge::new("registry").map(BasicChallenge::with_utf8);

    assert_eq!(
        challenge
            .ok()
            .and_then(authentication_header::<BasicUnauthorized>),
        Some(HeaderValue::from_static(
            "Basic realm=\"registry\", charset=\"UTF-8\""
        ))
    );
}

#[test]
fn basic_challenge_rejects_empty_oversized_and_injected_realms() {
    let longest = "r".repeat(128);
    let too_long = "r".repeat(129);

    assert!(BasicChallenge::new(&longest).is_ok());
    assert_eq!(
        BasicChallenge::new(""),
        Err(BasicChallengeError::EmptyRealm)
    );
    assert_eq!(
        BasicChallenge::new(&too_long),
        Err(BasicChallengeError::RealmTooLong { actual_bytes: 129 })
    );
    assert_eq!(
        BasicChallenge::new("safe\r\nforged: value"),
        Err(BasicChallengeError::InvalidByte {
            byte_index: 4,
            byte: b'\r',
        })
    );
    assert_eq!(
        BasicChallenge::new("café"),
        Err(BasicChallengeError::InvalidByte {
            byte_index: 3,
            byte: 0xc3,
        })
    );
    for rejected in ["realm\0", "realm\t", "realm\n", "realm\r", "realm\u{7f}"] {
        assert!(matches!(
            BasicChallenge::new(rejected),
            Err(BasicChallengeError::InvalidByte { .. })
        ));
    }
}

#[test]
fn worst_case_valid_basic_realm_remains_one_header_value() {
    let challenge = BasicChallenge::new(&"\\".repeat(128)).map(BasicChallenge::with_utf8);
    let header = challenge
        .ok()
        .and_then(authentication_header::<BasicUnauthorized>);

    assert!(header.is_some_and(|header| {
        header.as_bytes().iter().all(u8::is_ascii)
            && !header.as_bytes().contains(&b'\r')
            && !header.as_bytes().contains(&b'\n')
            && header.as_bytes().starts_with(b"Basic realm=\"")
            && header.as_bytes().ends_with(b"\", charset=\"UTF-8\"")
    }));
}

#[test]
fn bearer_challenge_behavior_is_unchanged_by_the_module_split() {
    let challenge = BearerChallenge::new("dispatch \"jobs\"");

    assert_eq!(
        challenge
            .ok()
            .and_then(authentication_header::<BearerUnauthorized>),
        Some(HeaderValue::from_static(
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
