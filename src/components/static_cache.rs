//! Aggressive HTTP caching for static assets (CSS, JS, images, fonts, media).
//!
//! [`static_asset_cache_middleware`] is installed by [`crate::http::into_axum_router`] so every
//! app and component route inherits the same policy: successful GET/HEAD responses whose
//! `Content-Type` is a static asset are stored for a year (`immutable`). Compile-time vendor
//! bundles also set these headers themselves (with content-hash URLs).
//!
//! Not cached: HTML, JSON/API payloads, PDFs, zip downloads, `Content-Disposition: attachment`,
//! and PWA service-worker / web manifest paths (those must revalidate so updates deploy).

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

/// Year-long immutable cache. Safe because hashed query params (`?v=`) or unique object
/// URLs (`/media/{id}`) change when the bytes change; other static MIME types are treated
/// as long-lived as well (PWA icons, form backgrounds, public CSS/JS/images).
pub const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// Service worker and web manifest must be revalidated on every navigation.
pub const MUST_REVALIDATE_CACHE_CONTROL: &str = "no-cache";

/// Insert [`IMMUTABLE_CACHE_CONTROL`] on `headers`.
pub fn apply_immutable_cache_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(IMMUTABLE_CACHE_CONTROL),
    );
}

/// Paths that must never be stored long-term (PWA update checks).
pub fn is_must_revalidate_path(path: &str) -> bool {
    matches!(path, "/serviceworker.js" | "/app.webmanifest")
}

/// True when `content_type` (with or without charset) is a cacheable static asset.
pub fn is_static_asset_content_type(content_type: &str) -> bool {
    let ct = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    matches!(
        ct.as_str(),
        "text/css"
            | "text/javascript"
            | "application/javascript"
            | "application/ecmascript"
            | "application/wasm"
            | "image/x-icon"
            | "image/vnd.microsoft.icon"
            | "application/font-woff"
            | "application/font-woff2"
            | "application/vnd.ms-fontobject"
    ) || ct.starts_with("image/")
        || ct.starts_with("font/")
        || ct.starts_with("audio/")
        || ct.starts_with("video/")
}

fn is_attachment(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_ascii_lowercase().contains("attachment"))
}

fn content_type_essence(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or("").trim())
}

/// True when this response should receive year-long immutable caching.
pub fn is_cacheable_static_response(path: &str, response: &Response) -> bool {
    if is_must_revalidate_path(path) {
        return false;
    }
    let status = response.status();
    if !(status.is_success() || status == StatusCode::NOT_MODIFIED) {
        return false;
    }
    if is_attachment(response.headers()) {
        return false;
    }
    content_type_essence(response.headers()).is_some_and(is_static_asset_content_type)
}

/// Skip `Set-Cookie` on cacheable static responses so browsers are allowed to store them.
pub fn should_omit_csrf_cookie(path: &str, response: &Response) -> bool {
    is_must_revalidate_path(path) || is_cacheable_static_response(path, response)
}

/// Stamp `Cache-Control` on static GET/HEAD responses that do not already set it.
///
/// Installed by [`crate::http::into_axum_router`].
pub async fn static_asset_cache_middleware(req: Request, next: Next) -> Response {
    let cacheable_method = matches!(*req.method(), Method::GET | Method::HEAD);
    if !cacheable_method {
        return next.run(req).await;
    }
    let path = req.uri().path().to_owned();
    let mut response = next.run(req).await;

    if is_must_revalidate_path(&path) {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(MUST_REVALIDATE_CACHE_CONTROL),
        );
        return response;
    }

    if response.headers().contains_key(header::CACHE_CONTROL) {
        return response;
    }

    if is_cacheable_static_response(&path, &response) {
        apply_immutable_cache_headers(response.headers_mut());
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, header};
    use axum::middleware::from_fn;
    use axum::response::IntoResponse;
    use axum::routing::get;
    use tower::ServiceExt;

    fn cache_control(res: &axum::http::Response<Body>) -> Option<&str> {
        res.headers()
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
    }

    #[test]
    fn classifies_static_mime_types() {
        assert!(is_static_asset_content_type("text/css; charset=utf-8"));
        assert!(is_static_asset_content_type("text/javascript"));
        assert!(is_static_asset_content_type("application/javascript"));
        assert!(is_static_asset_content_type("image/png"));
        assert!(is_static_asset_content_type("image/svg+xml"));
        assert!(is_static_asset_content_type("image/x-icon"));
        assert!(is_static_asset_content_type("font/woff2"));
        assert!(is_static_asset_content_type("video/webm"));
        assert!(is_static_asset_content_type("audio/mpeg"));
        assert!(is_static_asset_content_type("application/wasm"));
        assert!(!is_static_asset_content_type("text/html; charset=utf-8"));
        assert!(!is_static_asset_content_type("application/json"));
        assert!(!is_static_asset_content_type(
            "application/manifest+json; charset=utf-8"
        ));
        assert!(!is_static_asset_content_type("application/pdf"));
        assert!(!is_static_asset_content_type("application/zip"));
        assert!(!is_static_asset_content_type("application/octet-stream"));
        assert!(!is_static_asset_content_type("text/plain"));
    }

    fn app() -> Router {
        Router::new()
            .route(
                "/bundle.css",
                get(|| async {
                    (
                        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
                        "body{}",
                    )
                }),
            )
            .route(
                "/page",
                get(|| async {
                    (
                        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                        "<html></html>",
                    )
                }),
            )
            .route(
                "/api",
                get(|| async { ([(header::CONTENT_TYPE, "application/json")], "{}") }),
            )
            .route(
                "/icon.png",
                get(|| async { ([(header::CONTENT_TYPE, "image/png")], vec![0_u8, 1, 2]) }),
            )
            .route(
                "/download.png",
                get(|| async {
                    (
                        [
                            (header::CONTENT_TYPE, "image/png"),
                            (
                                header::CONTENT_DISPOSITION,
                                "attachment; filename=\"x.png\"",
                            ),
                        ],
                        vec![0_u8, 1, 2],
                    )
                }),
            )
            .route(
                "/already.css",
                get(|| async {
                    (
                        [
                            (header::CONTENT_TYPE, "text/css"),
                            (header::CACHE_CONTROL, "private, max-age=60"),
                        ],
                        "a{}",
                    )
                }),
            )
            .route(
                "/serviceworker.js",
                get(|| async {
                    (
                        [(
                            header::CONTENT_TYPE,
                            "application/javascript; charset=utf-8",
                        )],
                        "/* sw */",
                    )
                }),
            )
            .route(
                "/app.webmanifest",
                get(|| async {
                    (
                        [(
                            header::CONTENT_TYPE,
                            "application/manifest+json; charset=utf-8",
                        )],
                        "{}",
                    )
                }),
            )
            .route(
                "/missing.css",
                get(|| async { StatusCode::NOT_FOUND.into_response() }),
            )
            .layer(from_fn(static_asset_cache_middleware))
    }

    async fn get_path(path: &str) -> axum::http::Response<Body> {
        app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn caches_css_js_images_and_skips_html_json_attachments() {
        let css = get_path("/bundle.css").await;
        assert_eq!(css.status(), StatusCode::OK);
        assert_eq!(cache_control(&css), Some(IMMUTABLE_CACHE_CONTROL));

        let img = get_path("/icon.png").await;
        assert_eq!(cache_control(&img), Some(IMMUTABLE_CACHE_CONTROL));

        let html = get_path("/page").await;
        assert!(cache_control(&html).is_none());

        let json = get_path("/api").await;
        assert!(cache_control(&json).is_none());

        let download = get_path("/download.png").await;
        assert!(cache_control(&download).is_none());

        let missing = get_path("/missing.css").await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert!(cache_control(&missing).is_none());
    }

    #[tokio::test]
    async fn preserves_existing_cache_control_and_forces_revalidate_on_pwa_entrypoints() {
        let already = get_path("/already.css").await;
        assert_eq!(cache_control(&already), Some("private, max-age=60"));

        let sw = get_path("/serviceworker.js").await;
        assert_eq!(cache_control(&sw), Some(MUST_REVALIDATE_CACHE_CONTROL));

        let manifest = get_path("/app.webmanifest").await;
        assert_eq!(
            cache_control(&manifest),
            Some(MUST_REVALIDATE_CACHE_CONTROL)
        );
    }
}
