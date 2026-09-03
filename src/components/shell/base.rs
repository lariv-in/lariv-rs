//! Root HTML document with cached vendor bundles (HTMX 4, Alpine, DaisyUI, Tailwind).

use maud::{DOCTYPE, Markup, PreEscaped, html};

use super::vendor::vendor_head;

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
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                (vendor_head())
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
