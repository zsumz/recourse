//! Exact validation for catalog type bases and derived absolute URIs.

use fluent_uri::Uri;

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
