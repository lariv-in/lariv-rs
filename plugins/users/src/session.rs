use axum::http::{HeaderMap, header};
use chrono::Duration;

use lariv_core::web::{clear_cookie_header, cookie_value_from_headers, set_cookie_header};

pub const AUTH_COOKIE: &str = "auth-token";
/// User id a superuser is acting as. Ignored unless the auth cookie is a superuser.
pub const EFFECTIVE_USER_COOKIE: &str = "effective-user";
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

pub fn set_effective_user_cookie(headers: &mut HeaderMap, user_id: i64, secure: bool) {
    let value = set_cookie_header(
        EFFECTIVE_USER_COOKIE,
        &user_id.to_string(),
        SESSION_TTL.num_seconds(),
        secure,
    );
    headers.append(header::SET_COOKIE, value);
}

pub fn clear_effective_user_cookie(headers: &mut HeaderMap, secure: bool) {
    let value = clear_cookie_header(EFFECTIVE_USER_COOKIE, secure);
    headers.append(header::SET_COOKIE, value);
}

pub fn auth_token_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_value_from_headers(headers, AUTH_COOKIE)
}

pub fn effective_user_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_value_from_headers(headers, EFFECTIVE_USER_COOKIE)
}
