//! Vertical timeline list (Go `Timeline` port).

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::swap::hx_nav_app_layout_for_url;

pub struct TimelineItem {
    pub content: Markup,
    pub href: Option<String>,
}

pub struct Timeline<'a> {
    pub uid: &'a str,
    pub title: Option<&'a str>,
    pub actions: Markup,
    /// Rendered before the header (e.g. date filter forms).
    pub prefix: Markup,
    pub items: &'a [TimelineItem],
    pub pagination: Markup,
    pub classes: &'a str,
}

impl Default for Timeline<'_> {
    fn default() -> Self {
        Self {
            uid: "timeline-container",
            title: None,
            actions: Markup::default(),
            prefix: Markup::default(),
            items: &[],
            pagination: Markup::default(),
            classes: "",
        }
    }
}

fn timeline_item_link(href: &str, item: &TimelineItem) -> Markup {
    let attrs = hx_nav_app_layout_for_url(href).set("class", "block no-underline text-inherit");
    html! {
        (PreEscaped(format!(
            r#"<a href="{}"{}>"#,
            escape_attr(href),
            attrs.as_string()
        )))
        (timeline_item_row(item))
        (PreEscaped("</a>"))
    }
}

fn timeline_item_row(item: &TimelineItem) -> Markup {
    html! {
        div class="timeline-item relative flex items-center gap-4" {
            div class="timeline-indicator relative z-10 flex items-center" {
                div class="w-3 h-3 rounded-full bg-primary" {}
            }
            div class="timeline-card flex-1 p-2 m-1 rounded-box border border-base-300" {
                (item.content)
            }
        }
    }
}

/// Render a vertical timeline with optional header, connector line, and pagination.
pub fn timeline(opts: Timeline<'_>) -> Markup {
    let uid = if opts.uid.is_empty() {
        "timeline-container"
    } else {
        opts.uid
    };
    let show_header = opts.title.is_some();
    let show_line = !opts.items.is_empty();
    let empty = opts.items.is_empty();

    html! {
        div id=(uid) class=(format!("timeline-container {}", opts.classes)) {
            (opts.prefix)
            @if show_header {
                div class="flex justify-between items-center mb-4" {
                    @if let Some(title) = opts.title {
                        div class="text-xl font-semibold" { (title) }
                    }
                    div class="flex items-center gap-2" { (opts.actions) }
                }
            }
            div class="timeline-scroll relative" {
                @if show_line {
                    div class="absolute left-[5px] top-0 bottom-0 w-0.5 bg-primary opacity-30" {}
                }
                @if empty {
                    div class="text-center opacity-60 py-8" { "No items found" }
                } @else {
                    @for item in opts.items {
                        @if let Some(ref href) = item.href {
                            (timeline_item_link(href, item))
                        } @else {
                            (timeline_item_row(item))
                        }
                    }
                }
            }
            (opts.pagination)
        }
    }
}
