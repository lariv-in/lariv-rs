//! Label wrappers around child markup.

use maud::{Markup, PreEscaped, html};

pub struct LabelInline<'a> {
    pub label: &'a str,
    pub hint: Option<&'a str>,
    pub children: Markup,
}

/// `<div class="flex gap-2"><span class="text-primary font-bold">Title:</span>…</div>`
pub fn label_inline(label: &str, children: Markup) -> Markup {
    label_inline_hint(label, None, children)
}

pub fn label_inline_hint(label: &str, hint: Option<&str>, children: Markup) -> Markup {
    label_inline_with_classes_hint(label, "", hint, children)
}

pub fn label_inline_with_classes(label: &str, classes: &str, children: Markup) -> Markup {
    label_inline_with_classes_hint(label, classes, None, children)
}

pub fn label_inline_with_classes_hint(
    label: &str,
    classes: &str,
    hint: Option<&str>,
    children: Markup,
) -> Markup {
    html! {
        div class=(format!("flex gap-2 {classes}")) {
            (label_title(label, hint))
            (children)
        }
    }
}

pub struct LabelNewline<'a> {
    pub label: &'a str,
    pub hint: Option<&'a str>,
    pub children: Markup,
}

/// `<div class="my-1"><label class="label text-sm font-bold flex flex-col …">Title …</label></div>`
pub fn label_newline(label: &str, children: Markup) -> Markup {
    label_newline_hint(label, None, children)
}

pub fn label_newline_hint(label: &str, hint: Option<&str>, children: Markup) -> Markup {
    html! {
        div class="my-1 w-full" {
            label class="label text-sm font-bold flex flex-col items-stretch gap-1 w-full min-w-0" {
                span class="inline-flex items-center gap-1" {
                    (label)
                    (label_hint_icon(hint))
                }
                (children)
            }
        }
    }
}

fn label_title(label: &str, hint: Option<&str>) -> Markup {
    html! {
        span class="text-primary font-bold inline-flex items-center gap-1" {
            (label) ":"
            (label_hint_icon(hint))
        }
    }
}

fn label_hint_icon(hint: Option<&str>) -> Markup {
    match hint.filter(|text| !text.is_empty()) {
        Some(text) => {
            let body = html! { (text) }.into_string();
            PreEscaped(format!(
                r#"<span class="relative inline-flex align-middle" x-data="{{ hintOpen: false, hintLeave: null }}" x-on:mouseenter="clearTimeout(hintLeave); hintOpen = true" x-on:mouseleave="hintLeave = setTimeout(() => hintOpen = false, 120)"><span class="cursor-help opacity-60 text-sm font-normal leading-none" aria-label="Field information">🛈</span><div x-show="hintOpen" x-cloak class="absolute top-full left-0 pt-1 z-50 w-max max-w-lg"><div class="max-h-64 overflow-y-auto overflow-x-hidden rounded-box border border-base-300 bg-base-100 text-base-content px-3 py-2 text-left text-xs leading-snug whitespace-pre-line shadow-md">{body}</div></div></span>"#
            ))
        }
        None => html! {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maud::html;

    fn markup(m: Markup) -> String {
        m.into_string()
    }

    #[test]
    fn label_inline_without_hint_omits_icon() {
        let html = markup(label_inline("Status", html! { "ok" }));
        assert!(html.contains("Status:"));
        assert!(!html.contains("🛈"));
        assert!(!html.contains("data-tip"));
        assert!(!html.contains("hintOpen"));
    }

    #[test]
    fn label_inline_with_hint_renders_tooltip_icon() {
        let html = markup(label_inline_hint(
            "Reference",
            Some("Shown on the PDF invoice."),
            html! { "INV-001" },
        ));
        assert!(html.contains("Reference:"));
        assert!(html.contains("🛈"));
        assert!(html.contains("hintOpen"));
        assert!(html.contains("max-h-64 overflow-y-auto"));
        assert!(html.contains("top-full left-0"));
        assert!(html.contains("Shown on the PDF invoice."));
    }

    #[test]
    fn label_newline_with_hint_renders_tooltip_icon() {
        let html = markup(label_newline_hint(
            "Notes",
            Some("Internal only."),
            html! { "Example" },
        ));
        assert!(html.contains("label text-sm font-bold"));
        assert!(html.contains("Notes"));
        assert!(!html.contains("Notes:"));
        assert!(html.contains("🛈"));
        assert!(html.contains("hintOpen"));
        assert!(html.contains("Internal only."));
    }
}
