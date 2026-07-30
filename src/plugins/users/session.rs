use axum::http::{HeaderMap, header};
use axum_extra::extract::cookie::Cookie;
use chrono::Duration;

use crate::web::{clear_cookie_header, set_cookie_header};

pub const AUTH_COOKIE: &str = "auth-token";
pub const SESSION_TTL: Duration = Duration::hours(24);

pub fn is_secure_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
}

pub fn set_auth_cookie(headers: &mut HeaderMap, token: &str, secure: bool) {
    let value = set_cookie_header(
        AUTH_COOKIE,
        token,
        SESSION_TTL.num_seconds(),
        secure,
    );
    headers.append(header::SET_COOKIE, value);
}

pub fn clear_auth_cookie(headers: &mut HeaderMap, secure: bool) {
    let value = clear_cookie_header(AUTH_COOKIE, secure);
    headers.append(header::SET_COOKIE, value);
}

pub fn auth_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&format!("{AUTH_COOKIE}=")) {
            return Some(value.to_string());
        }
    }
    None
}

pub fn build_auth_cookie(token: &str, secure: bool) -> Cookie<'static> {
    let mut cookie = Cookie::build((AUTH_COOKIE, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(SESSION_TTL.num_seconds()))
        .build();
    if secure {
        cookie.set_secure(true);
    }
    cookie
}
