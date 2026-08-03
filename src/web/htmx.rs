//! HTMX 4 request context, axum extractor, and response middleware.
//!
//! Parses HTMX headers into [`Htmx`], exposes swap-target helpers, and rewrites 3xx
//! redirects to `200` + `HX-Redirect` for HTMX clients. Installed globally via
//! [`crate::http::into_axum_router`].
//!
//! # Routes
//!
//! Handlers extract [`Htmx`] as a parameter. Partial responses use
//! [`RenderAppPane`](crate::template::RenderAppPane) via [`crate::web::html_page_or_app_layout`]
//! or [`crate::layers::render_from_data`].
//!
//! # Use cases
//!
//! - Branch between full page, `#app-layout`, and `#main-content` responses.
//! - Issue redirects that work for both HTMX and plain navigation.
//! - Detect boosted navigation and history-restore (must return full pages).
//!
//! # Examples
//!
//! ```rust ignore
//! async fn edit_user(htmx: Htmx, /* ... */) -> impl IntoResponse {
//!     if htmx.wants_app_layout() {
//!         page.render_pane()
//!     } else {
//!         page.render(&chrome)
//!     }
//! }
//! ```

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    body::Body,
};
use axum::http::Request;

use crate::components::swap::{AppLayoutKey, MainContentKey, SwapKey};

/// HTMX request classification from the `HX-Request-Type` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmxRequestType {
    /// Full document swap.
    Full,
    /// Partial region swap (pane, table, modal target, etc.).
    Partial,
}

impl HtmxRequestType {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "full" => Some(Self::Full),
            "partial" => Some(Self::Partial),
            _ => None,
        }
    }
}

/// Parsed HTMX request headers (always present; `request` is `false` for normal navigations).
///
/// Populated by [`htmx_middleware`] into request extensions, or parsed on demand when
/// used as an axum extractor.
#[derive(Debug, Clone, Default)]
pub struct Htmx {
    /// `true` when `HX-Request: true`.
    pub request: bool,
    /// `true` when `HX-Boosted: true` (link/form boost).
    pub boosted: bool,
    /// Back/forward navigation — server must return a full page (HTMX 4 re-fetch).
    pub history_restore: bool,
    /// `Full` or `Partial` from `HX-Request-Type`, if present.
    pub request_type: Option<HtmxRequestType>,
    /// Parsed element id from `HX-Target` (e.g. `"app-layout"`).
    pub target_id: Option<String>,
    /// Parsed element id from `HX-Source` / legacy `HX-Trigger`.
    pub source_id: Option<String>,
    /// Current browser URL from `HX-Current-URL`.
    pub current_url: Option<String>,
}

impl Htmx {
    /// Build from raw request headers (used by middleware and extractor fallback).
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let request = header_true(headers, "HX-Request");
        let boosted = header_true(headers, "HX-Boosted");
        let history_restore = header_true(headers, "HX-History-Restore-Request");
        let request_type = headers
            .get("HX-Request-Type")
            .and_then(|v| v.to_str().ok())
            .and_then(HtmxRequestType::parse);
        let target_id = headers
            .get("HX-Target")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_element_id);
        let source_id = headers
            .get("HX-Source")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_element_id)
            .or_else(|| {
                // HTMX 2 sent HX-Trigger; accept during mixed clients.
                headers
                    .get("HX-Trigger")
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_element_id)
            });
        let current_url = headers
            .get("HX-Current-URL")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        Self {
            request,
            boosted,
            history_restore,
            request_type,
            target_id,
            source_id,
            current_url,
        }
    }

    /// `true` for partial swaps; `false` on history restore even when `HX-Request` is set.
    pub fn wants_partial(&self) -> bool {
        self.request && !self.history_restore
    }

    /// True when HTMX is targeting the given [`SwapKey`] region.
    pub fn targets<K: SwapKey>(&self) -> bool {
        self.target_id.as_deref() == Some(K::ID)
    }

    /// `true` when HTMX targets the app layout pane (`#app-layout` / [`AppLayoutKey`]).
    pub fn wants_app_layout(&self) -> bool {
        self.wants_partial() && self.targets::<AppLayoutKey>()
    }

    /// `true` when HTMX targets `<main id="main-content">` ([`MainContentKey`]).
    pub fn wants_main_content(&self) -> bool {
        self.wants_partial() && self.targets::<MainContentKey>()
    }

    /// HTMX-aware redirect: `200` + `HX-Redirect` for HTMX requests, else 303 `Location`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use lariv_rs::web::Htmx;
    /// # use axum::http::StatusCode;
    /// let htmx = Htmx { request: true, ..Default::default() };
    /// let res = htmx.redirect("/done/");
    /// assert_eq!(res.status(), StatusCode::OK);
    /// ```
    pub fn redirect(&self, path: &str) -> Response {
        if self.request {
            let mut response = StatusCode::OK.into_response();
            if let Ok(value) = HeaderValue::from_str(path) {
                response.headers_mut().insert("HX-Redirect", value);
            }
            response
        } else {
            Redirect::to(path).into_response()
        }
    }
}

impl<S> FromRequestParts<S> for Htmx
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(htmx) = parts.extensions.get::<Htmx>() {
            return Ok(htmx.clone());
        }
        Ok(Htmx::from_headers(&parts.headers))
    }
}

/// Parse an element id from HTMX 4 `tag#id`, `#id`, or bare `id`.
///
/// # Examples
///
/// ```rust
/// # use lariv_rs::web::parse_element_id;
/// assert_eq!(parse_element_id("div#app-layout").as_deref(), Some("app-layout"));
/// assert_eq!(parse_element_id("#user-table").as_deref(), Some("user-table"));
/// ```
pub fn parse_element_id(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some((_, id)) = raw.rsplit_once('#') {
        let id = id.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_owned())
        }
    } else if let Some(id) = raw.strip_prefix('#') {
        let id = id.trim();
        if id.is_empty() {
            None
        } else {
            Some(id.to_owned())
        }
    } else {
        Some(raw.to_owned())
    }
}

fn header_true(headers: &HeaderMap, name: &str) -> bool {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

const VARY_HTMX: &str = "HX-Request, HX-Target, HX-Request-Type, HX-History-Restore-Request";

/// Insert [`Htmx`] into extensions; rewrite 3xx `Location` → `200` `HX-Redirect`; set `Vary`.
///
/// Applied automatically by [`crate::http::into_axum_router`]. Non-HTMX requests pass through
/// unchanged except for extension insertion.
pub async fn htmx_middleware(mut req: Request<Body>, next: Next) -> Response {
    let htmx = Htmx::from_headers(req.headers());
    let is_htmx = htmx.request;
    req.extensions_mut().insert(htmx);

    let mut response = next.run(req).await;

    if !is_htmx {
        return response;
    }

    response.headers_mut().insert(
        header::VARY,
        HeaderValue::from_static(VARY_HTMX),
    );

    let status = response.status();
    if status.is_redirection()
        && let Some(location) = response.headers().get(header::LOCATION).cloned()
    {
        let mut headers = response.headers().clone();
        headers.remove(header::LOCATION);
        headers.insert("HX-Redirect", location);
        headers.insert(header::VARY, HeaderValue::from_static(VARY_HTMX));
        let body = response.into_body();
        let mut rebuilt = Response::new(body);
        *rebuilt.status_mut() = StatusCode::OK;
        *rebuilt.headers_mut() = headers;
        return rebuilt;
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swap_key;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::{Router, response::Redirect};
    use tower::ServiceExt;

    swap_key!(TestPaneKey, "app-layout");
    swap_key!(TestTableKey, "user-table");

    #[test]
    fn parse_element_id_formats() {
        assert_eq!(
            parse_element_id("div#app-layout").as_deref(),
            Some("app-layout")
        );
        assert_eq!(
            parse_element_id("#app-layout").as_deref(),
            Some("app-layout")
        );
        assert_eq!(
            parse_element_id("app-layout").as_deref(),
            Some("app-layout")
        );
        assert_eq!(parse_element_id("div#").as_deref(), None);
        assert_eq!(parse_element_id("").as_deref(), None);
    }

    #[test]
    fn history_restore_requests_full_page_not_pane() {
        let mut headers = HeaderMap::new();
        headers.insert("HX-Request", HeaderValue::from_static("true"));
        headers.insert(
            "HX-History-Restore-Request",
            HeaderValue::from_static("true"),
        );
        headers.insert("HX-Target", HeaderValue::from_static("div#app-layout"));
        let htmx = Htmx::from_headers(&headers);
        assert!(htmx.history_restore);
        assert!(!htmx.wants_partial());
        assert!(!htmx.wants_app_layout());
        assert!(!htmx.wants_main_content());
    }

    #[test]
    fn targets_and_wants_app_layout() {
        let mut headers = HeaderMap::new();
        headers.insert("HX-Request", HeaderValue::from_static("true"));
        headers.insert("HX-Target", HeaderValue::from_static("div#app-layout"));
        let htmx = Htmx::from_headers(&headers);
        assert!(htmx.request);
        assert!(htmx.targets::<TestPaneKey>());
        assert!(htmx.wants_app_layout());
        assert!(!htmx.targets::<TestTableKey>());
    }

    #[test]
    fn redirect_htmx_uses_200_hx_redirect() {
        let htmx = Htmx {
            request: true,
            ..Default::default()
        };
        let response = htmx.redirect("/users/");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("HX-Redirect").and_then(|v| v.to_str().ok()),
            Some("/users/")
        );
        assert!(response.headers().get(header::LOCATION).is_none());
    }

    #[test]
    fn redirect_non_htmx_uses_303() {
        let htmx = Htmx::default();
        let response = htmx.redirect("/users/");
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()),
            Some("/users/")
        );
    }

    #[tokio::test]
    async fn middleware_rewrites_3xx_to_hx_redirect() {
        let app = Router::new()
            .route("/go", get(|| async { Redirect::to("/users/login") }))
            .layer(from_fn(htmx_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/go")
                    .header("HX-Request", "true")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("HX-Redirect").and_then(|v| v.to_str().ok()),
            Some("/users/login")
        );
        assert!(response.headers().get(header::LOCATION).is_none());
        assert_eq!(
            response.headers().get(header::VARY).and_then(|v| v.to_str().ok()),
            Some(VARY_HTMX)
        );
    }

    #[tokio::test]
    async fn middleware_leaves_non_htmx_redirect() {
        let app = Router::new()
            .route("/go", get(|| async { Redirect::to("/users/login") }))
            .layer(from_fn(htmx_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/go")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()),
            Some("/users/login")
        );
    }
}
