//! Label wrappers around child markup.

use maud::{Markup, html};

pub struct LabelInline<'a> {
    pub label: &'a str,
    pub children: Markup,
}

/// `<div class="flex gap-2"><span class="text-primary font-bold">Title:</span>…</div>`
pub fn label_inline(label: &str, children: Markup) -> Markup {
    label_inline_with_classes(label, "", children)
}

pub fn label_inline_with_classes(label: &str, classes: &str, children: Markup) -> Markup {
    html! {
        div class=(format!("flex gap-2 {classes}")) {
            span class="text-primary font-bold" { (label) ":" }
            (children)
        }
    }
}

pub struct LabelNewline<'a> {
    pub label: &'a str,
    pub children: Markup,
}

pub fn label_newline(label: &str, children: Markup) -> Markup {
    html! {
        label class="label text-sm font-bold flex flex-col items-start gap-1" {
            (label)
            (children)
        }
    }
}
