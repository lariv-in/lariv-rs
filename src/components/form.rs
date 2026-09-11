//! Form wrapper (presentational — parsing stays in handlers).

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::html_form::{CsrfToken, csrf_hidden_field};

pub struct FormOpts<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub action: Option<&'a str>,
    pub method: &'a str,
    pub enctype: Option<&'a str>,
    pub classes: &'a str,
    pub form_error: Option<&'a str>,
    pub inputs: Markup,
    pub actions: Markup,
    pub prefix: Markup,
    pub suffix: Markup,
    pub attrs: HtmlAttrs,
}

impl Default for FormOpts<'_> {
    fn default() -> Self {
        Self {
            title: "",
            subtitle: "",
            action: None,
            method: "post",
            enctype: None,
            classes: "",
            form_error: None,
            inputs: Markup::default(),
            actions: Markup::default(),
            prefix: Markup::default(),
            suffix: Markup::default(),
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn form(csrf: &CsrfToken, opts: FormOpts<'_>) -> Markup {
    let class = format!("flex flex-col gap-2 {}", opts.classes);
    let mut open = format!(r#"<form class="{}""#, escape_attr(&class));
    // Prefer hx-post/hx-get from attrs; only emit method when attrs do not set it.
    if !opts.attrs.attrs.contains_key("method") {
        open.push_str(&format!(r#" method="{}""#, escape_attr(opts.method)));
    }
    if let Some(action) = opts.action {
        // Skip duplicate action= when hx-post/hx-get already carry the URL.
        if !opts.attrs.attrs.contains_key("hx-post") && !opts.attrs.attrs.contains_key("hx-get") {
            open.push_str(&format!(r#" action="{}""#, escape_attr(action)));
        }
    }
    if let Some(enctype) = opts.enctype {
        open.push_str(&format!(r#" enctype="{}""#, escape_attr(enctype)));
    }
    open.push_str(&opts.attrs.as_string());
    open.push('>');
    let csrf = if form_submits_mutation(&opts) {
        csrf_hidden_field(csrf)
    } else {
        Markup::default()
    };
    html! {
        (PreEscaped(open))
        (csrf)
        (opts.prefix)
        // Go always emits this wrapper div even when title/subtitle are empty.
        div {
            @if !opts.title.is_empty() {
                div class="text-xl font-semibold" { (opts.title) }
            }
            @if !opts.subtitle.is_empty() {
                div class="text-sm text-gray-500" { (opts.subtitle) }
            }
        }
        div { (opts.inputs) }
        @if let Some(err) = opts.form_error {
            @if !err.is_empty() {
                span class="text-sm text-error" { (err) }
            }
        }
        div class="my-2 flex w-full justify-between items-center" { (opts.actions) }
        (opts.suffix)
        (PreEscaped("</form>"))
    }
}

fn form_submits_mutation(opts: &FormOpts<'_>) -> bool {
    if opts.attrs.attrs.contains_key("hx-get") {
        return false;
    }
    if opts.attrs.attrs.contains_key("hx-post") {
        return true;
    }
    matches!(
        opts.method.to_ascii_lowercase().as_str(),
        "post" | "put" | "patch" | "delete"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html_form::{CSRF_FIELD, CsrfToken, generate_csrf_token, scope_csrf};

    #[tokio::test]
    async fn post_form_emits_csrf_field() {
        let token = generate_csrf_token();
        let html = scope_csrf(token.clone(), async {
            form(
                &CsrfToken::current(),
                FormOpts {
                    method: "post",
                    inputs: Markup::default(),
                    ..Default::default()
                },
            )
            .into_string()
        })
        .await;
        assert!(html.contains(&format!(r#"name="{CSRF_FIELD}""#)), "{html}");
        assert!(html.contains(&format!(r#"value="{token}""#)), "{html}");
    }

    #[tokio::test]
    async fn get_form_omits_csrf_field() {
        let token = generate_csrf_token();
        let html = scope_csrf(token, async {
            form(
                &CsrfToken::current(),
                FormOpts {
                    method: "get",
                    attrs: HtmlAttrs::new().set("hx-get", "/items"),
                    inputs: Markup::default(),
                    ..Default::default()
                },
            )
            .into_string()
        })
        .await;
        assert!(!html.contains(CSRF_FIELD), "{html}");
    }
}
