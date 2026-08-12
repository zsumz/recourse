//! Exact validation for catalog type bases and derived absolute URIs.

use fluent_uri::Uri;

use crate::wire::WireLimits;

const MAX_CODE_NUMBER_DECIMAL_BYTES: usize = 10;

pub(crate) fn valid_type_base(value: &str) -> bool {
    let Ok(uri) = Uri::parse(value) else {
        return false;
    };
    if uri.query().is_some() || uri.fragment().is_some() || !uri.path().as_str().ends_with('/') {
        return false;
    }
    let scheme = uri.scheme().as_str();
    if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
        uri.authority().is_some() && uri.path().as_str().starts_with('/')
    } else {
        true
    }
}

pub(crate) fn valid_type_uri(value: &str) -> bool {
    Uri::parse(value).is_ok()
}

pub(crate) fn maximum_type_uri_bytes(type_base: &str, prefix: &str) -> usize {
    type_base
        .len()
        .saturating_add(prefix.len())
        .saturating_add(1 + MAX_CODE_NUMBER_DECIMAL_BYTES)
}

pub(crate) fn type_namespace_fits_wire(type_base: &str, prefix: &str) -> bool {
    maximum_type_uri_bytes(type_base, prefix) <= WireLimits::DEFAULT_MAX_STRING_BYTES
}
