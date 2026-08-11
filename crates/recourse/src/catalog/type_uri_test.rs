//! Absolute type-base validation independent of HTTP request-target grammar.

use super::{valid_type_base, valid_type_uri};

#[test]
fn type_bases_are_absolute_path_bases_without_query_or_fragment() {
    for valid in [
        "https://example.invalid/problems/",
        "recourse://dispatch/problems/",
        "example:problems/",
    ] {
        assert!(valid_type_base(valid), "expected valid type base {valid:?}");
    }
    for invalid in [
        "problems/",
        "https:/problems/",
        "https://example.invalid/problems?next=/",
        "https://example.invalid/problems#/",
        "https://example.invalid/problems",
    ] {
        assert!(
            !valid_type_base(invalid),
            "expected invalid type base {invalid:?}"
        );
    }
}

#[test]
fn derived_type_identity_must_remain_an_absolute_uri() {
    assert!(valid_type_uri("https://example.invalid/problems/EXM-1"));
    assert!(valid_type_uri("recourse://dispatch/problems/DSP-1003"));
    assert!(!valid_type_uri("/problems/EXM-1"));
}
