//! Published wire-limit discriminant stability.

use super::WireLimit;

#[test]
fn additive_limits_do_not_renumber_existing_variants() {
    for (limit, discriminant) in [
        (WireLimit::BodyBytes, 0),
        (WireLimit::NestingDepth, 1),
        (WireLimit::ObjectProperties, 2),
        (WireLimit::ArrayItems, 3),
        (WireLimit::StringBytes, 4),
        (WireLimit::Suggestions, 5),
        (WireLimit::Violations, 6),
        (WireLimit::NumberBytes, 7),
    ] {
        assert_eq!(limit as usize, discriminant);
    }
}
