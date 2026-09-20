use axum::http::{HeaderMap, header};
use chrono::Duration;

use crate::web::{clear_cookie_header, cookie_value_from_headers, set_cookie_header};

pub const AUTH_COOKIE: &str = "auth-token";
pub const SESSION_TTL: Duration = Duration::hours(24);

pub fn is_secure_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
}

pub fn set_auth_cookie(headers: &mut HeaderMap, token: &str, secure: bool) {
    let value = set_cookie_header(AUTH_COOKIE, token, SESSION_TTL.num_seconds(), secure);
    headers.append(header::SET_COOKIE, value);
}

pub fn clear_auth_cookie(headers: &mut HeaderMap, secure: bool) {
    let value = clear_cookie_header(AUTH_COOKIE, secure);
    headers.append(header::SET_COOKIE, value);
}

pub fn auth_token_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_value_from_headers(headers, AUTH_COOKIE)
}
