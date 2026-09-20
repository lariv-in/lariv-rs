//! Signed `meets-anon` cookie mapping guests to [`super::entities::AnonymousUser`] rows.

use axum::http::{HeaderMap, HeaderValue};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::plugins::users::session::is_secure_request;
use crate::web::{clear_cookie_header, cookie_value_from_headers, set_cookie_header};

pub const ANON_COOKIE: &str = "meets-anon";
const ANON_TTL_SECS: i64 = 60 * 60 * 24 * 30;

pub fn sign_anon_id(secret: &[u8], id: i64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(b"|");
    hasher.update(id.to_le_bytes());
    let hash = hasher.finalize();
    let sig = URL_SAFE_NO_PAD.encode(&hash[..16]);
    format!("{id}.{sig}")
}

pub fn parse_anon_id(secret: &[u8], value: &str) -> Option<i64> {
    let (id_str, _) = value.split_once('.')?;
    let id: i64 = id_str.parse().ok()?;
    let expected = sign_anon_id(secret, id);
    if expected == value { Some(id) } else { None }
}

pub fn anon_id_from_headers(headers: &HeaderMap, secret: &[u8]) -> Option<i64> {
    let value = cookie_value_from_headers(headers, ANON_COOKIE)?;
    parse_anon_id(secret, &value)
}

pub fn set_anon_cookie_header(secret: &[u8], id: i64, headers: &HeaderMap) -> HeaderValue {
    let token = sign_anon_id(secret, id);
    set_cookie_header(
        ANON_COOKIE,
        &token,
        ANON_TTL_SECS,
        is_secure_request(headers),
    )
}

pub fn clear_anon_cookie_header(headers: &HeaderMap) -> HeaderValue {
    clear_cookie_header(ANON_COOKIE, is_secure_request(headers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_anon_cookie() {
        let secret = b"test-secret";
        let token = sign_anon_id(secret, 42);
        assert_eq!(parse_anon_id(secret, &token), Some(42));
        assert_eq!(parse_anon_id(b"other", &token), None);
        assert_eq!(parse_anon_id(secret, "42.deadbeef"), None);
    }
}
