//! Destructive delete confirmation form (Go `DeleteConfirmation` port).

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::button::{ButtonSubmit, button_submit};

pub struct DeleteConfirmation<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub classes: &'a str,
    pub form_error: Option<&'a str>,
    pub global_error: Option<&'a str>,
    pub attrs: HtmlAttrs,
}

impl Default for DeleteConfirmation<'_> {
    fn default() -> Self {
        Self {
            title: "Confirm Deletion",
            message: "Are you sure you want to delete this item?",
            classes: "",
            form_error: None,
            global_error: None,
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn delete_confirmation(opts: DeleteConfirmation<'_>) -> Markup {
    let form_class = "flex flex-col gap-2 gap-2 my-4";
    html! {
        div class=(format!("container mx-auto {}", opts.classes)) {
            h2 class="text-xl font-bold text-error" { (opts.title) }
            p class="my-2" { (opts.message) }
            @if let Some(err) = opts.global_error {
                @if !err.is_empty() {
                    div class="alert alert-error my-2 text-sm" { (err) }
                }
            }
            (PreEscaped(format!(
                r#"<form class="{}"{}>"#,
                escape_attr(form_class),
                opts.attrs.as_string()
            )))
            @if let Some(err) = opts.form_error {
                @if !err.is_empty() {
                    span class="text-sm text-error" { (err) }
                }
            }
            div class="my-2" {
                (button_submit(ButtonSubmit {
                    label: "Confirm Delete",
                    classes: "btn-error my-2",
                    ..Default::default()
                }))
            }
            (PreEscaped("</form>"))
        }
    }
}
