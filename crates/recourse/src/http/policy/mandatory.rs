//! Status-derived response headers that no policy may bypass.

const WWW_AUTHENTICATE: &[&str] = &["www-authenticate"];
const ALLOW: &[&str] = &["allow"];
const PROXY_AUTHENTICATE: &[&str] = &["proxy-authenticate"];
const UPGRADE: &[&str] = &["upgrade"];
const NONE: &[&str] = &[];

/// Returns headers mandated by the semantics of an HTTP status.
pub(crate) const fn mandatory_headers(status: u16) -> &'static [&'static str] {
    match status {
        401 => WWW_AUTHENTICATE,
        405 => ALLOW,
        407 => PROXY_AUTHENTICATE,
        426 => UPGRADE,
        _ => NONE,
    }
}
