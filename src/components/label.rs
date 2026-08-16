//! Label wrappers around child markup.

use maud::{Markup, PreEscaped, html};

pub struct Label<'a> {
    pub label: &'a str,
    pub hint: Option<&'a str>,
    pub children: Markup,
}

/// `<div class="my-1"><label class="label text-sm font-bold flex flex-col …">Title …</label></div>`
pub fn label(label: &str, children: Markup) -> Markup {
    label_hint(label, None, children)
}

pub fn label_hint(label: &str, hint: Option<&str>, children: Markup) -> Markup {
    html! {
        div class="my-1 w-full" {
            label class="label text-sm font-bold flex flex-col items-stretch gap-1 w-full min-w-0" {
                span class="inline-flex items-center gap-1" {
                    (label)
                    (label_hint_icon(hint))
                }
            }
                (children)
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
    fn label_without_hint_omits_icon() {
        let html = markup(label("Status", html! { "ok" }));
        assert!(html.contains("Status"));
        assert!(!html.contains("Status:"));
        assert!(!html.contains("🛈"));
        assert!(!html.contains("data-tip"));
        assert!(!html.contains("hintOpen"));
    }

    #[test]
    fn label_with_hint_renders_tooltip_icon() {
        let html = markup(label_hint(
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
