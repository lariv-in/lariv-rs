//! DaisyUI breadcrumb trail for app-pane navigation chrome.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::nav_origin::{DASHBOARD_URL, from_dashboard};
use crate::components::swap::hx_nav_app_layout_for_url;

/// One segment in a breadcrumb trail.
///
/// `href: None` marks the current page (rendered as plain text, no link).
#[derive(Clone, Copy, Debug)]
pub struct Crumb<'a> {
    pub label: &'a str,
    pub href: Option<&'a str>,
}

const DASHBOARD_CRUMB: Crumb<'static> = Crumb {
    label: "Dashboard",
    href: Some(DASHBOARD_URL),
};

/// Render a DaisyUI breadcrumb list. Empty `items` yields empty markup unless
/// the request arrived from the apps dashboard (then a Dashboard crumb is shown).
///
/// Linked crumbs navigate with `#app-layout` HTMX swaps (same as sidebar pane links).
pub fn breadcrumbs(items: &[Crumb<'_>]) -> Markup {
    let prepend_dashboard = should_prepend_dashboard(items);
    if items.is_empty() && !prepend_dashboard {
        return Markup::default();
    }
    html! {
        div class="breadcrumbs text-sm min-w-0 flex-1 overflow-x-auto" {
            ul {
                @if prepend_dashboard {
                    (crumb_li(&DASHBOARD_CRUMB))
                }
                @for item in items {
                    (crumb_li(item))
                }
            }
        }
    }
}

fn should_prepend_dashboard(items: &[Crumb<'_>]) -> bool {
    if !from_dashboard() {
        return false;
    }
    !items
        .first()
        .is_some_and(|c| c.label == DASHBOARD_CRUMB.label && c.href == DASHBOARD_CRUMB.href)
}

fn crumb_li(item: &Crumb<'_>) -> Markup {
    html! {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::nav_origin::scope_from_dashboard;

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
        assert!(!html.contains(">Dashboard</a>"));
    }

    #[tokio::test]
    async fn prepends_dashboard_when_origin_set() {
        scope_from_dashboard(true, async {
            let html = breadcrumbs(&[Crumb {
                label: "Blog",
                href: Some("/blog/"),
            }])
            .into_string();
            #[cfg(feature = "plugin-dashboard")]
            {
                assert!(html.contains("hx-get=\"/dashboard/\""));
                assert!(html.contains(">Dashboard</a>"));
                let dash = html.find(">Dashboard</a>").expect("dashboard crumb");
                let blog = html.find(">Blog</a>").expect("blog crumb");
                assert!(dash < blog);
            }
            #[cfg(not(feature = "plugin-dashboard"))]
            {
                assert!(!html.contains(">Dashboard</a>"));
            }
        })
        .await;
    }

    #[tokio::test]
    async fn empty_crumbs_show_dashboard_when_origin_set() {
        scope_from_dashboard(true, async {
            let html = breadcrumbs(&[]).into_string();
            #[cfg(feature = "plugin-dashboard")]
            {
                assert!(html.contains(r#"class="breadcrumbs"#));
                assert!(html.contains(">Dashboard</a>"));
                assert!(html.contains("hx-get=\"/dashboard/\""));
            }
            #[cfg(not(feature = "plugin-dashboard"))]
            {
                assert_eq!(html, "");
            }
        })
        .await;
    }

    #[tokio::test]
    async fn does_not_duplicate_existing_dashboard_crumb() {
        scope_from_dashboard(true, async {
            let html = breadcrumbs(&[DASHBOARD_CRUMB]).into_string();
            #[cfg(feature = "plugin-dashboard")]
            {
                assert_eq!(html.matches(">Dashboard</a>").count(), 1);
            }
        })
        .await;
    }
}
