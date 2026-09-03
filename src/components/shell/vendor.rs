//! Compile-time vendor snippets for the document shell (HTMX, Alpine, DaisyUI, Tailwind).
//!
//! CSS and JS are served as `/bundle.css` and `/bundle.js` so the HTML document stays
//! small and browsers can cache the vendor stack across navigations.
//!
//! Pinned sources (jsDelivr / unpkg / Fontshare / Google Fonts CSS):
//! - htmx.org@4.0.0-beta6 (`dist/htmx.min.js`, `dist/ext/hx-ws.min.js`,
//!   `dist/ext/hx-head.min.js`, `dist/ext/hx-alpine-compat.js`)
//! - @alpinejs/persist@3.16.2 (`dist/cdn.min.js`)
//! - alpinejs@3.17.1 (`dist/cdn.min.js`)
//! - daisyui@5.7.25 (`daisyui.css`)
//! - @tailwindcss/browser@4.3.3 (`dist/index.global.js`)
//! - apexcharts@7.1.0 (`dist/apexcharts.min.js`)
//! - Fontshare Satoshi CSS (`f[]=satoshi@300,400,500,600,700`)
//! - Google Fonts Roboto Mono CSS (`wght@400;500;600;700`)

use std::sync::LazyLock;

use axum::Router;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use maud::{Markup, PreEscaped, html};

/// Concatenated vendor CSS (fonts, icons, DaisyUI, theme extras).
pub const BUNDLE_CSS: &str = concat!(
    include_str!("vendor/satoshi.css"),
    "\n",
    include_str!("vendor/roboto-mono.css"),
    "\n",
    include_str!("vendor/heroicon.css"),
    "\n",
    include_str!("vendor/daisyui.css"),
    "\n",
    include_str!("vendor/theme.css"),
);

/// Concatenated vendor JS. Order matches the former inlined head + body stack:
/// HTMX and extensions, chrome helpers, Tailwind browser, then Alpine (DOM-ready via `defer`).
pub const BUNDLE_JS: &str = concat!(
    include_str!("vendor/htmx.min.js"),
    ";\n",
    include_str!("vendor/hx-ws.min.js"),
    ";\n",
    include_str!("vendor/hx-head.min.js"),
    ";\n",
    include_str!("vendor/toggle-theme.js"),
    ";\n",
    include_str!("vendor/date-picker.js"),
    ";\n",
    include_str!("vendor/tailwindcss-browser.js"),
    ";\n",
    include_str!("vendor/alpine-persist.min.js"),
    ";\n",
    include_str!("vendor/alpine.min.js"),
    ";\n",
    include_str!("vendor/hx-alpine-compat.js"),
    ";\n",
);

static BUNDLE_CSS_HASH: LazyLock<u64> = LazyLock::new(|| fnv1a_64(BUNDLE_CSS));
static BUNDLE_JS_HASH: LazyLock<u64> = LazyLock::new(|| fnv1a_64(BUNDLE_JS));

fn fnv1a_64(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Cache-busting stylesheet URL (`/bundle.css?v=…`).
pub fn bundle_css_href() -> String {
    format!("/bundle.css?v={:016x}", *BUNDLE_CSS_HASH)
}

/// Cache-busting script URL (`/bundle.js?v=…`).
pub fn bundle_js_href() -> String {
    format!("/bundle.js?v={:016x}", *BUNDLE_JS_HASH)
}

/// Head tags: HTMX config, cached CSS/JS links, Tailwind `@theme` (must stay a
/// `style[type="text/tailwindcss"]` for `@tailwindcss/browser`).
pub fn vendor_head() -> Markup {
    let css_href = bundle_css_href();
    let js_href = bundle_js_href();
    html! {
        (PreEscaped(
            r#"<meta name="htmx-config" content='{"defaultSwap":"outerHTML","noSwap":[204,304,"5xx"]}'>"#,
        ))
        link rel="stylesheet" href=(css_href);
        style type="text/tailwindcss" {
            (PreEscaped(include_str!("vendor/theme.css")))
        }
        script src=(js_href) defer {}
    }
}

/// Register `/bundle.css` and `/bundle.js` on the axum router.
pub fn mount_vendor_bundles(router: Router<()>) -> Router<()> {
    router
        .route("/bundle.css", get(bundle_css))
        .route("/bundle.js", get(bundle_js))
}

async fn bundle_css() -> Response {
    static_asset(BUNDLE_CSS, "text/css; charset=utf-8", *BUNDLE_CSS_HASH)
}

async fn bundle_js() -> Response {
    static_asset(BUNDLE_JS, "text/javascript; charset=utf-8", *BUNDLE_JS_HASH)
}

fn static_asset(body: &'static str, content_type: &'static str, hash: u64) -> Response {
    let etag = format!("\"{hash:016x}\"");
    let mut response = (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        body,
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}

/// ApexCharts runtime, skipped if `window.ApexCharts` is already defined.
pub const fn apexcharts_script() -> PreEscaped<&'static str> {
    PreEscaped(concat!(
        "<script>if(typeof ApexCharts==='undefined'){",
        include_str!("vendor/apexcharts.min.js"),
        "}</script>",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[test]
    fn bundle_css_contains_vendor_styles() {
        assert!(BUNDLE_CSS.contains("daisyUI 5.7.25"));
        assert!(BUNDLE_CSS.contains("font-family: 'Satoshi'"));
        assert!(BUNDLE_CSS.contains("font-family: 'Roboto Mono'"));
        assert!(BUNDLE_CSS.contains("[x-cloak]"));
        assert!(!BUNDLE_CSS.contains("cdn.jsdelivr.net"));
    }

    #[test]
    fn date_picker_iso_regex_is_terminated() {
        let src = include_str!("vendor/date-picker.js");
        assert!(
            !src.contains(r#"(?::(\d{2})?)/"#),
            "unterminated parenthetical in larivTextToIso ISO datetime regex"
        );
        assert!(src.contains(r#"(\d{2}:\d{2}(?::(\d{2}))?)"#));
    }

    #[test]
    fn bundle_js_contains_vendor_scripts() {
        assert!(BUNDLE_JS.contains("var htmx="));
        assert!(BUNDLE_JS.contains("hx-ws"));
        assert!(BUNDLE_JS.contains("hx-head"));
        assert!(BUNDLE_JS.contains("hx-alpine-compat"));
        assert!(BUNDLE_JS.contains("$persist"));
        assert!(BUNDLE_JS.contains("4.3.3"));
        assert!(!BUNDLE_JS.contains("ApexCharts"));
    }

    #[test]
    fn vendor_head_links_hashed_bundles() {
        let html = vendor_head().into_string();
        assert!(html.contains(&bundle_css_href()));
        assert!(html.contains(&bundle_js_href()));
        assert!(html.contains(r#"name="htmx-config""#));
        assert!(html.contains(r#"type="text/tailwindcss""#));
        assert!(html.contains("defer"));
        assert!(
            html.contains("</script>"),
            "external <script src> must be closed or the browser treats the rest of the document as JS: {html}"
        );
        assert!(!html.contains("var htmx="));
        assert!(!html.contains("daisyUI 5.7.25"));
    }

    #[tokio::test]
    async fn bundle_routes_serve_cached_assets() {
        let app = mount_vendor_bundles(Router::new());

        let css = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/bundle.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(css.status(), StatusCode::OK);
        assert_eq!(
            css.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/css; charset=utf-8")
        );
        assert_eq!(
            css.headers()
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok()),
            Some("public, max-age=31536000, immutable")
        );
        let css_body = String::from_utf8(
            axum::body::to_bytes(css.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(css_body.contains("daisyUI 5.7.25"));

        let js = app
            .oneshot(
                Request::builder()
                    .uri("/bundle.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(js.status(), StatusCode::OK);
        assert_eq!(
            js.headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("text/javascript; charset=utf-8")
        );
        let js_body = String::from_utf8(
            axum::body::to_bytes(js.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(js_body.contains("var htmx="));
    }
}
