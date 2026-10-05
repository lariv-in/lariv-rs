//! Layout containers.

use maud::{Markup, html};

pub struct ContainerRow {
    pub classes: &'static str,
    pub children: Markup,
}

pub fn container_row(classes: &str, children: Markup) -> Markup {
    html! {
        div class=(format!("flex flex-row gap-1 {}", classes)) { (children) }
    }
}

pub struct ContainerColumn {
    pub classes: &'static str,
    pub children: Markup,
}

pub fn container_column(classes: &str, children: Markup) -> Markup {
    html! {
        div class=(format!("flex flex-col gap-1 min-w-0 w-full {}", classes)) { (children) }
    }
}

pub struct ContainerError {
    pub error: Option<&'static str>,
    pub children: Markup,
}

pub fn container_error(error: Option<&str>, children: Markup) -> Markup {
    html! {
        div class="flex flex-col gap-1 w-full" {
            (children)
            @if let Some(msg) = error {
                @if !msg.is_empty() {
                    span class="text-sm text-error" { (msg) }
                }
            }
        }
    }
}

pub struct ContainerHtml {
    pub classes: &'static str,
    pub children: Markup,
}

pub fn container_html(classes: &str, children: Markup) -> Markup {
    html! {
        div class=(classes) { (children) }
    }
}
