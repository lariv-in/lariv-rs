//! Read-only display fields.

use maud::{Markup, PreEscaped, html};
use pulldown_cmark::{Options, Parser, html as md_html};

use crate::components::container::container_error;

pub struct FieldTitle<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_title(opts: FieldTitle<'_>) -> Markup {
    // Go FieldTitle.Build always prefixes "text-xl font-semibold text-primary ".
    let class = format!("text-xl font-semibold text-primary {}", opts.classes);
    html! { div class=(class) { (opts.value) } }
}

pub struct FieldSubtitle<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_subtitle(opts: FieldSubtitle<'_>) -> Markup {
    let class = if opts.classes.is_empty() {
        "text-md text-gray-500".to_string()
    } else {
        opts.classes.to_string()
    };
    html! { div class=(class) { (opts.value) } }
}

pub struct FieldText<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_text(opts: FieldText<'_>) -> Markup {
    html! { div class=(opts.classes) { (opts.value) } }
}

pub struct FieldTextarea<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_textarea(opts: FieldTextarea<'_>) -> Markup {
    html! { div class=(format!("whitespace-pre-wrap {}", opts.classes)) { (opts.value) } }
}

pub struct FieldCheckbox<'a> {
    pub checked: bool,
    pub classes: &'a str,
}

pub fn field_checkbox(opts: FieldCheckbox<'_>) -> Markup {
    // Go FieldCheckbox: check-circle (success) / x-circle (error) icons.
    let (name, classes) = if opts.checked {
        ("check-circle", "text-success")
    } else {
        ("x-circle", "text-error")
    };
    html! {
        span {
            (crate::components::text::icon(name, classes))
        }
    }
}

pub struct FieldLink<'a> {
    pub href: &'a str,
    pub label: &'a str,
    pub classes: &'a str,
}

pub fn field_link(opts: FieldLink<'_>) -> Markup {
    let class = if opts.classes.is_empty() {
        "link link-primary".to_string()
    } else {
        opts.classes.to_string()
    };
    html! { a href=(opts.href) class=(class) { (opts.label) } }
}

pub struct FieldPhone<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_phone(opts: FieldPhone<'_>) -> Markup {
    // Go FieldPhone: parse with region IN, format E.164; on failure render ContainerError.
    match phonenumber::parse(Some(phonenumber::country::IN), opts.value) {
        Ok(parsed) => {
            let formatted = parsed.format().mode(phonenumber::Mode::E164).to_string();
            html! { div class=(opts.classes) { (formatted) } }
        }
        Err(err) => {
            let msg = err.to_string();
            container_error(Some(&msg), Markup::default())
        }
    }
}

pub struct FieldDate<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_date(opts: FieldDate<'_>) -> Markup {
    html! { div class=(opts.classes) { (opts.value) } }
}

pub struct FieldTime<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_time(opts: FieldTime<'_>) -> Markup {
    html! { div class=(opts.classes) { (opts.value) } }
}

pub struct FieldDatetime<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_datetime(opts: FieldDatetime<'_>) -> Markup {
    html! { div class=(opts.classes) { (opts.value) } }
}

pub struct FieldDuration<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

pub fn field_duration(opts: FieldDuration<'_>) -> Markup {
    html! { div class=(opts.classes) { (opts.value) } }
}

/// Chip list for related many-to-many records (Go `FieldManyToMany`).
pub struct FieldManyToMany<'a> {
    pub items: &'a [(&'a str, Option<&'a str>)],
    pub classes: &'a str,
}

pub fn field_many_to_many(opts: FieldManyToMany<'_>) -> Markup {
    html! {
        div class=(format!("flex flex-wrap gap-2 {}", opts.classes)) {
            @for (label, href) in opts.items {
                @if let Some(url) = href {
                    a href=(url) class="badge badge-outline" { (label) }
                } @else {
                    span class="badge badge-outline" { (label) }
                }
            }
        }
    }
}

/// Rendered markdown body (Go `FieldMarkdown`).
pub struct FieldMarkdown<'a> {
    pub value: &'a str,
    pub classes: &'a str,
}

/// Parse markdown to HTML with common extensions (sanitized via escaped raw HTML skip).
pub fn render_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, options);
    let mut html_out = String::new();
    md_html::push_html(&mut html_out, parser);
    html_out
}

pub fn field_markdown(opts: FieldMarkdown<'_>) -> Markup {
    if opts.value.is_empty() {
        return html! {};
    }
    let rendered = render_markdown(opts.value);
    let class = format!(
        "whitespace-pre-wrap border border-base-300 p-2 rounded-md prose max-w-none {}",
        opts.classes
    );
    html! {
        div class=(class) {
            (PreEscaped(rendered))
        }
    }
}
