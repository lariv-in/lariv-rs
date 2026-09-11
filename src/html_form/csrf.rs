//! Double-submit CSRF for [`super::HtmlForm`]: cookie plus hidden form field.
//!
//! [`csrf_middleware`] issues a `csrf_token` cookie and scopes the token for
//! rendering. [`crate::components::form`] and [`super::FormCtx::form`] require a
//! [`CsrfToken`] so a form cannot be built without one. [`super::HtmlFormBody`]
//! and [`super::HtmlForm::from_multipart`] reject POSTs unless the cookie and
//! field match.

use std::cell::OnceCell;
use std::future::Future;

use axum::{
    extract::{FromRequestParts, Request},
    http::{HeaderMap, StatusCode, header, request::Parts},
    middleware::Next,
    response::Response,
};
use maud::{Markup, html};
use rand::RngCore;

use super::{FormError, UrlencodedFields};
use crate::web::set_cookie_header;

/// Cookie and hidden-field name for the CSRF token.
pub const CSRF_COOKIE: &str = "csrf_token";
/// HTML `name` of the hidden CSRF input (same value as [`CSRF_COOKIE`]).
pub const CSRF_FIELD: &str = "csrf_token";

const CSRF_TTL_SECS: i64 = 24 * 60 * 60;
const TOKEN_BYTES: usize = 32;

tokio::task_local! {
    static CSRF_TOKEN: String;
}

std::thread_local! {
    static FALLBACK_CSRF: OnceCell<String> = const { OnceCell::new() };
}

/// Owned CSRF token required to render or parse an [`super::HtmlForm`].
///
/// Construct with [`CsrfToken::current`] in handlers and templates (middleware
/// scopes the request token). Empty [`Default`] is only for tests and
/// `#[serde(default)]` on generated form structs.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Token for this request: task-local from [`csrf_middleware`], else a
    /// stable per-thread fallback so tests without middleware still compile.
    pub fn current() -> Self {
        if let Ok(t) = CSRF_TOKEN.try_with(|t| t.clone()) {
            return Self(t);
        }
        FALLBACK_CSRF.with(|c| Self(c.get_or_init(generate_csrf_token).clone()))
    }

    /// Prefer the scoped request token, then the CSRF cookie, then [`Self::current`].
    pub fn from_headers(headers: &HeaderMap) -> Self {
        if let Ok(t) = CSRF_TOKEN.try_with(|t| t.clone()) {
            return Self(t);
        }
        csrf_token_from_headers(headers)
            .map(Self)
            .unwrap_or_else(Self::current)
    }

    /// Token bytes as a string (cookie / hidden-field value).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True when this is the empty [`Default`] token.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Compare this token to a submitted form field (constant-time).
    pub fn verify_field(&self, field: Option<&str>) -> Result<(), FormError> {
        let cookie = (!self.is_empty()).then_some(self.as_str());
        verify_csrf(cookie, field)
    }
}

impl AsRef<str> for CsrfToken {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<S> FromRequestParts<S> for CsrfToken
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self::from_headers(&parts.headers))
    }
}

/// Random hex token suitable for the CSRF cookie and hidden field.
pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    to_hex(&bytes)
}

/// CSRF token for the current request, if [`csrf_middleware`] (or [`scope_csrf`]) is active.
pub fn current_csrf_token() -> Option<String> {
    CSRF_TOKEN.try_with(|t| t.clone()).ok()
}

/// Hidden `<input>` carrying `csrf`. Always emitted; pass [`CsrfToken::current`].
pub fn csrf_hidden_field(csrf: &CsrfToken) -> Markup {
    html! {
        input type="hidden" name=(CSRF_FIELD) value=(csrf.as_str()) autocomplete="off";
    }
}

/// Run `fut` with `token` as [`current_csrf_token`].
pub async fn scope_csrf<F>(token: String, fut: F) -> F::Output
where
    F: Future,
{
    CSRF_TOKEN.scope(token, fut).await
}

/// Read the CSRF cookie from a `Cookie` header.
pub fn csrf_token_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_value(headers, CSRF_COOKIE).filter(|t| is_plausible_token(t))
}

/// Compare the CSRF cookie to the hidden field on a parsed form body.
pub fn verify_form_csrf(headers: &HeaderMap, fields: &UrlencodedFields) -> Result<(), FormError> {
    verify_csrf(
        csrf_token_from_headers(headers).as_deref(),
        fields.get_first(CSRF_FIELD),
    )
}

/// Compare cookie token to submitted field (constant-time).
pub fn verify_csrf(cookie: Option<&str>, field: Option<&str>) -> Result<(), FormError> {
    let cookie = cookie.filter(|s| !s.is_empty()).ok_or(FormError::Csrf)?;
    let field = field.filter(|s| !s.is_empty()).ok_or(FormError::Csrf)?;
    if !tokens_match(cookie, field) {
        return Err(FormError::Csrf);
    }
    Ok(())
}

/// `403` rejection for [`FormError::Csrf`]; `None` for other form errors.
pub fn csrf_rejection(err: &FormError) -> Option<(StatusCode, String)> {
    match err {
        FormError::Csrf => Some((
            StatusCode::FORBIDDEN,
            "CSRF token missing or invalid".into(),
        )),
        _ => None,
    }
}

/// Issue / reuse the CSRF cookie and scope [`current_csrf_token`] for the request.
///
/// Installed by [`crate::http::into_axum_router`].
pub async fn csrf_middleware(req: Request, next: Next) -> Response {
    let secure = is_secure_request(req.headers());
    let incoming = csrf_token_from_headers(req.headers());
    let set_cookie = incoming.is_none();
    let token = incoming.unwrap_or_else(generate_csrf_token);

    let mut response = scope_csrf(token.clone(), async move { next.run(req).await }).await;
    if set_cookie {
        set_csrf_cookie(response.headers_mut(), &token, secure);
    }
    response
}

fn set_csrf_cookie(headers: &mut HeaderMap, token: &str, secure: bool) {
    headers.append(
        header::SET_COOKIE,
        set_cookie_header(CSRF_COOKIE, token, CSRF_TTL_SECS, secure),
    );
}

fn is_secure_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("https"))
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{name}=");
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }
    None
}

fn is_plausible_token(s: &str) -> bool {
    let n = s.len();
    (32..=128).contains(&n) && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn tokens_match(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;
    use axum::middleware::from_fn;
    use axum::routing::{get, post};
    use axum::{Router, body::Body};
    use tower::ServiceExt;

    #[test]
    fn generated_token_is_plausible() {
        let t = generate_csrf_token();
        assert!(is_plausible_token(&t), "{t}");
        assert_eq!(t.len(), TOKEN_BYTES * 2);
    }

    #[test]
    fn verify_csrf_requires_matching_non_empty_tokens() {
        assert!(verify_csrf(None, Some("abc")).is_err());
        assert!(verify_csrf(Some("abc"), None).is_err());
        assert!(verify_csrf(Some(""), Some("x")).is_err());
        assert!(verify_csrf(Some("token-a"), Some("token-b")).is_err());
        assert!(verify_csrf(Some("same-token-value-ok"), Some("same-token-value-ok")).is_ok());
    }

    #[test]
    fn cookie_parser_reads_csrf_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "auth-token=abc; csrf_token=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            csrf_token_from_headers(&headers).as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }

    #[test]
    fn hidden_field_always_emits_token_value() {
        let csrf = CsrfToken::default();
        let html = csrf_hidden_field(&csrf).into_string();
        assert!(html.contains(&format!(r#"name="{CSRF_FIELD}""#)), "{html}");
        assert!(html.contains(r#"type="hidden""#), "{html}");
        assert!(html.contains(r#"value="""#), "{html}");
    }

    #[tokio::test]
    async fn hidden_field_emits_scoped_token() {
        let token = generate_csrf_token();
        let html = scope_csrf(token.clone(), async {
            csrf_hidden_field(&CsrfToken::current()).into_string()
        })
        .await;
        assert!(html.contains(&format!(r#"name="{CSRF_FIELD}""#)), "{html}");
        assert!(html.contains(&format!(r#"value="{token}""#)), "{html}");
        assert!(html.contains(r#"type="hidden""#), "{html}");
    }

    #[tokio::test]
    async fn middleware_sets_cookie_and_scopes_token() {
        let app = Router::new()
            .route(
                "/",
                get(|| async { current_csrf_token().unwrap_or_default() }),
            )
            .layer(from_fn(csrf_middleware));

        let response = app
            .oneshot(HttpRequest::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            set_cookie.contains(&format!("{CSRF_COOKIE}=")),
            "{set_cookie}"
        );
        assert!(set_cookie.contains("HttpOnly"), "{set_cookie}");
        assert!(set_cookie.contains("SameSite=Lax"), "{set_cookie}");

        let token = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(is_plausible_token(&token), "{token}");
        assert!(set_cookie.contains(&token), "{set_cookie}");
    }

    #[tokio::test]
    async fn middleware_reuses_existing_cookie() {
        let token = generate_csrf_token();
        let app = Router::new()
            .route(
                "/",
                post(|| async { current_csrf_token().unwrap_or_default() }),
            )
            .layer(from_fn(csrf_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/")
                    .header(header::COOKIE, format!("{CSRF_COOKIE}={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.headers().get(header::SET_COOKIE).is_none());
        let body = String::from_utf8(
            axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert_eq!(body, token);
    }
}
