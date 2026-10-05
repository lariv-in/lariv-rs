//! Root HTML document with cached vendor bundles (HTMX 4, Alpine, DaisyUI, Tailwind).

use std::sync::RwLock;

use maud::{DOCTYPE, Markup, PreEscaped, html};

use super::vendor::vendor_head;

static APP_MANIFEST_PATH: RwLock<String> = RwLock::new(String::new());
static FAVICON_PATH: RwLock<String> = RwLock::new(String::new());
static APPLE_PWA_HEAD: RwLock<String> = RwLock::new(String::new());

fn read_nonempty(lock: &RwLock<String>) -> Option<String> {
    lock.read()
        .ok()
        .map(|g| g.clone())
        .filter(|s| !s.is_empty())
}

fn write_lock(lock: &RwLock<String>, value: impl Into<String>) {
    if let Ok(mut guard) = lock.write() {
        *guard = value.into();
    }
}

/// Current `<link rel="manifest">` href patched in by the PWA plugin.
///
/// Empty when no plugin has called [`set_app_manifest_path`].
pub fn app_manifest_path() -> Option<String> {
    read_nonempty(&APP_MANIFEST_PATH)
}

/// Patch the app manifest path into [`shell_base`] (empty string removes the link).
pub fn set_app_manifest_path(path: impl Into<String>) {
    write_lock(&APP_MANIFEST_PATH, path);
}

/// Current `<link rel="icon">` href patched in by the PWA plugin.
///
/// Empty when no plugin has called [`set_favicon_path`].
pub fn favicon_path() -> Option<String> {
    read_nonempty(&FAVICON_PATH)
}

/// Patch the favicon path into [`shell_base`] (empty string removes the link).
pub fn set_favicon_path(path: impl Into<String>) {
    write_lock(&FAVICON_PATH, path);
}

/// Apple iOS PWA `<head>` snippet patched in by the PWA plugin.
pub fn apple_pwa_head() -> Option<String> {
    read_nonempty(&APPLE_PWA_HEAD)
}

/// Patch Apple iOS PWA meta/link tags into [`shell_base`] (empty string removes them).
pub fn set_apple_pwa_head(html: impl Into<String>) {
    write_lock(&APPLE_PWA_HEAD, html);
}

fn attr_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Combined PWA `<head>` snippet: favicon, manifest link, and Apple iOS tags.
///
/// Empty when none have been patched. Safe to splice into HTML as-is.
pub fn pwa_head_html() -> String {
    let mut out = String::new();
    if let Some(href) = favicon_path() {
        out.push_str(r#"<link rel="icon" href=""#);
        out.push_str(&attr_escape(&href));
        out.push_str("\">");
    }
    if let Some(href) = app_manifest_path() {
        out.push_str(r#"<link rel="manifest" href=""#);
        out.push_str(&attr_escape(&href));
        out.push_str("\">");
    }
    if let Some(apple) = apple_pwa_head() {
        out.push_str(&apple);
    }
    out
}

/// Arguments for the root HTML document shell.
pub struct ShellBase<'a> {
    pub title: &'a str,
    pub registry_head: Markup,
    pub extra_head: Markup,
    pub body: Markup,
    pub global_error: Option<&'a str>,
}

impl Default for ShellBase<'_> {
    fn default() -> Self {
        Self {
            title: "Lariv",
            registry_head: Markup::default(),
            extra_head: Markup::default(),
            body: Markup::default(),
            global_error: None,
        }
    }
}

/// Render the full HTML document (doctype, cached vendor bundles, body chrome).
pub fn shell_base(opts: ShellBase<'_>) -> Markup {
    let body_inner = shell_base_body(opts.body, opts.global_error);
    let favicon = favicon_path();
    let manifest = app_manifest_path();
    let apple = apple_pwa_head();
    let viewport = if apple.is_some() {
        "width=device-width, initial-scale=1.0, viewport-fit=cover"
    } else {
        "width=device-width, initial-scale=1.0"
    };
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content=(viewport);
                (vendor_head())
                @if let Some(href) = favicon.as_deref() {
                    link rel="icon" href=(href);
                }
                @if let Some(href) = manifest.as_deref() {
                    link rel="manifest" href=(href);
                }
                @if let Some(apple) = apple.as_deref() {
                    (PreEscaped(apple))
                }
                (opts.registry_head)
                (opts.extra_head)
                // Document `<title>` comes from head slots (`CoreTitle` / PWA patch),
                // matching Go Catalog HeadNodes — not from `opts.title`.
            }
            (body_inner)
        }
    }
}

fn shell_base_body(children: Markup, global_error: Option<&str>) -> Markup {
    // HTMX 4: swap/indicator use `:inherited`; navigation targets are explicit
    // on each link/form (see `hx_nav_app_layout_for_url`, `nav_main_attrs`, etc.).
    html! {
        (PreEscaped(
            r##"<body class="hide-right font-sans" x-data="{ theme: localStorage.getItem('theme') || 'light' }" :data-theme="theme" hx-swap:inherited="outerHTML" hx-indicator:inherited="#global-loading-indicator">"##,
        ))
        div id="global-loading-indicator" class="fixed top-0 left-0 w-full z-50" {
            div class="h-0.5 bg-primary animate-pulse" {}
        }
        (children)
        @if let Some(err) = global_error {
            @if !err.is_empty() {
                div class="toast toast-bottom toast-center z-50" {
                    div class="alert alert-error" { (err) }
                }
            }
        }
        (PreEscaped("</body>"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ManifestPathGuard(Option<String>);

    impl ManifestPathGuard {
        fn set(path: &str) -> Self {
            let previous = app_manifest_path();
            set_app_manifest_path(path);
            Self(previous)
        }
    }

    impl Drop for ManifestPathGuard {
        fn drop(&mut self) {
            set_app_manifest_path(self.0.take().unwrap_or_default());
        }
    }

    struct FaviconPathGuard(Option<String>);

    impl FaviconPathGuard {
        fn set(path: &str) -> Self {
            let previous = favicon_path();
            set_favicon_path(path);
            Self(previous)
        }
    }

    impl Drop for FaviconPathGuard {
        fn drop(&mut self) {
            set_favicon_path(self.0.take().unwrap_or_default());
        }
    }

    struct AppleHeadGuard(Option<String>);

    impl AppleHeadGuard {
        fn set(html: &str) -> Self {
            let previous = apple_pwa_head();
            set_apple_pwa_head(html);
            Self(previous)
        }
    }

    impl Drop for AppleHeadGuard {
        fn drop(&mut self) {
            set_apple_pwa_head(self.0.take().unwrap_or_default());
        }
    }

    #[test]
    fn shell_base_emits_manifest_link_only_when_patched() {
        let _guard = ManifestPathGuard::set("");
        let html = shell_base(ShellBase {
            body: html! { p { "hi" } },
            ..Default::default()
        })
        .into_string();
        assert!(
            !html.contains(r#"rel="manifest""#),
            "unpatched shell_base must not emit a manifest link: {html}"
        );

        set_app_manifest_path("/app.webmanifest");
        let html = shell_base(ShellBase {
            body: html! { p { "hi" } },
            ..Default::default()
        })
        .into_string();
        assert!(
            html.contains(r#"rel="manifest""#),
            "patched shell_base must emit a manifest link: {html}"
        );
        assert!(
            html.contains(r#"href="/app.webmanifest""#),
            "patched shell_base must use the route path: {html}"
        );
    }

    #[test]
    fn shell_base_emits_favicon_link_only_when_patched() {
        let _guard = FaviconPathGuard::set("");
        let html = shell_base(ShellBase {
            body: html! { p { "hi" } },
            ..Default::default()
        })
        .into_string();
        assert!(
            !html.contains(r#"rel="icon""#),
            "unpatched shell_base must not emit a favicon link: {html}"
        );

        set_favicon_path("/favicon.ico");
        let html = shell_base(ShellBase {
            body: html! { p { "hi" } },
            ..Default::default()
        })
        .into_string();
        assert!(
            html.contains(r#"rel="icon""#),
            "patched shell_base must emit a favicon link: {html}"
        );
        assert!(
            html.contains(r#"href="/favicon.ico""#),
            "patched shell_base must use the route path: {html}"
        );
        assert!(
            pwa_head_html().contains(r#"<link rel="icon" href="/favicon.ico">"#),
            "pwa_head_html must include the favicon link"
        );
    }

    #[test]
    fn shell_base_emits_apple_pwa_head_only_when_patched() {
        let _guard = AppleHeadGuard::set("");
        let html = shell_base(ShellBase {
            body: html! { p { "hi" } },
            ..Default::default()
        })
        .into_string();
        assert!(
            !html.contains("apple-mobile-web-app-capable"),
            "unpatched shell_base must not emit Apple PWA tags: {html}"
        );
        assert!(
            html.contains(r#"content="width=device-width, initial-scale=1.0""#),
            "unpatched viewport must stay default: {html}"
        );
        assert!(
            !html.contains("viewport-fit=cover"),
            "unpatched viewport must not force cover: {html}"
        );

        set_apple_pwa_head(r#"<meta name="apple-mobile-web-app-capable" content="yes">"#);
        let html = shell_base(ShellBase {
            body: html! { p { "hi" } },
            ..Default::default()
        })
        .into_string();
        assert!(
            html.contains(r#"name="apple-mobile-web-app-capable""#),
            "patched shell_base must emit Apple PWA tags: {html}"
        );
        assert!(
            html.contains("viewport-fit=cover"),
            "iOS standalone viewport must include viewport-fit=cover: {html}"
        );
    }
}
