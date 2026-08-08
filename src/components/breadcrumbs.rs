//! DaisyUI breadcrumb trail for app-pane navigation chrome.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::swap::hx_nav_app_layout_for_url;

/// One segment in a breadcrumb trail.
///
/// `href: None` marks the current page (rendered as plain text, no link).
#[derive(Clone, Copy, Debug)]
pub struct Crumb<'a> {
    pub label: &'a str,
    pub href: Option<&'a str>,
}

/// Render a DaisyUI breadcrumb list. Empty `items` yields empty markup.
///
/// Linked crumbs navigate with `#app-layout` HTMX swaps (same as sidebar pane links).
pub fn breadcrumbs(items: &[Crumb<'_>]) -> Markup {
    if items.is_empty() {
        return Markup::default();
    }
    html! {
        div class="breadcrumbs text-sm min-w-0 flex-1 overflow-x-auto" {
            ul {
                @for item in items {
                    li {
                        @if let Some(href) = item.href {
                            (PreEscaped(format!(
                                r#"<a href="{url}"{attrs}>"#,
                                url = escape_attr(href),
                                attrs = hx_nav_app_layout_for_url(href).as_string(),
                            )))
                            (item.label)
                            (PreEscaped("</a>"))
                        } @else {
                            span { (item.label) }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_crumbs_render_nothing() {
        assert_eq!(breadcrumbs(&[]).into_string(), "");
    }

    #[test]
    fn linked_and_current_crumbs() {
        let html = breadcrumbs(&[
            Crumb {
                label: "Blog",
                href: Some("/blog/"),
            },
            Crumb {
                label: "Edit",
                href: None,
            },
        ])
        .into_string();
        assert!(html.contains(r#"class="breadcrumbs"#));
        assert!(html.contains("hx-target=\"#app-layout\""));
        assert!(html.contains("hx-get=\"/blog/\""));
        assert!(html.contains(">Blog</a>"));
        assert!(html.contains("<span>Edit</span>"));
        // Current crumb must not be a link.
        assert!(!html.contains(">Edit</a>"));
    }
}
