//! Root HTML document with CDN stack (HTMX 4, Alpine, DaisyUI, Tailwind).

use maud::{DOCTYPE, Markup, PreEscaped, html};

// Match Go `shell_base.go` string constants exactly (spacing included).
const HEROICON_CSS: &str = ".heroicon {display: inline-block;width: 24px;height: 24px;background-color: currentColor;-webkit-mask-image: var(--heroicon-url);mask-image: var(--heroicon-url);-webkit-mask-repeat: no-repeat;mask-repeat: no-repeat;-webkit-mask-size: 100% 100%;mask-size: 100% 100%;}.heroicon-sm {width: 16px;height: 16px;}.heroicon-lg {width: 32px;height: 32px;}";

const TOGGLE_THEME_JS: &str = "function toggleTheme() { const d = Alpine.$data(document.body); d.theme = d.theme === 'light' ? 'dark' : 'light'; localStorage.setItem('theme', d.theme); }";

/// HTMX 4 config: page navigations use outerHTML (see body `:inherited`); do not swap bare 5xx.
const HTMX_CONFIG_META: &str = r#"{"defaultSwap":"outerHTML","noSwap":[204,304,"5xx"]}"#;

const THEME_CSS: &str = r#"@theme {--font-sans: "Satoshi", ui-sans-serif, system-ui, sans-serif;--font-mono: "Roboto Mono", monospace;}:root {font-family: var(--font-sans);}[data-theme="dark"] {--color-base-100: oklch(14% 0.014 253);--color-base-200: oklch(24% 0.014 253);--color-base-300: oklch(30% 0.016 252);}#global-loading-indicator {opacity: 0;transition: opacity 200ms ease-in;}#global-loading-indicator.htmx-request {opacity: 1;}"#;

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

/// Render the full HTML document (doctype, head CDNs, body chrome).
pub fn shell_base(opts: ShellBase<'_>) -> Markup {
    let body_inner = shell_base_body(opts.body, opts.global_error);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                // HTMX 4 — exclusive; no htmx-2-compat.
                (PreEscaped(format!(
                    r#"<meta name="htmx-config" content='{}'>"#,
                    HTMX_CONFIG_META
                )))
                script
                    src="https://cdn.jsdelivr.net/npm/htmx.org@4.0.0-beta6"
                    integrity="sha384-6lyVbhrs13b9z7mLOpt/N6R76rtkEBWgCjAXRs/DSWyi2AMnQSs10ijWk+PI8n7W"
                    crossorigin="anonymous" {}
                // Alpine before alpine-compat so fragment init can see `window.Alpine`.
                script src="//unpkg.com/alpinejs" defer {}
                // Alpine fragment init (replaces alpine-morph / @alpinejs/morph).
                script
                    defer
                    src="https://cdn.jsdelivr.net/npm/htmx.org@4.0.0-beta6/dist/ext/hx-alpine-compat.js" {}
                (PreEscaped(
                    r#"<link href="https://api.fontshare.com/v2/css?f[]=satoshi@300,400,500,600,700&display=swap" rel="stylesheet">"#,
                ))
                (PreEscaped(
                    r#"<link href="https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;500;600;700&display=swap" rel="stylesheet">"#,
                ))
                style { (PreEscaped(HEROICON_CSS)) }
                script { (PreEscaped(TOGGLE_THEME_JS)) }
                link href="https://cdn.jsdelivr.net/npm/daisyui@5/daisyui.css" rel="stylesheet" type="text/css";
                script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4" {}
                style type="text/tailwindcss" { (PreEscaped(THEME_CSS)) }
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
    // HTMX 4: inheritance is explicit — boost/target/swap must use `:inherited`
    // so in-app links and forms pick up the `#app-layout` pane without per-link attrs.
    html! {
        (PreEscaped(
            r##"<body class="hide-right font-sans" x-data="{ theme: localStorage.getItem('theme') || 'light' }" :data-theme="theme" hx-boost:inherited="true" hx-target:inherited="#app-layout" hx-select:inherited="#app-layout" hx-swap:inherited="outerHTML" hx-indicator:inherited="#global-loading-indicator" hx-push-url:inherited="true">"##,
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
