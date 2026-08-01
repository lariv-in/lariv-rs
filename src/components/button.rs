//! Action buttons.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::swap::{form_hx_boost_post_main, hx_nav_app_layout_for_url};
use crate::http::RouteUrl;
use crate::components::text::icon;

fn is_external_href(href: &str) -> bool {
    href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
}

fn link_hx_attrs(href: &str) -> HtmlAttrs {
    if is_external_href(href) {
        HtmlAttrs::new().set("hx-boost", "false")
    } else if href.starts_with('/') {
        hx_nav_app_layout_for_url(href)
    } else {
        HtmlAttrs::new()
    }
}

pub struct ButtonSubmit<'a> {
    pub label: &'a str,
    pub icon_name: Option<&'a str>,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonSubmit<'_> {
    fn default() -> Self {
        Self {
            label: "Submit",
            icon_name: None,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_submit(opts: ButtonSubmit<'_>) -> Markup {
    let mut class = format!("btn btn-primary {}", opts.classes);
    if opts.icon_name.is_some() && !opts.label.is_empty() {
        class.push_str(" inline-flex items-center gap-2");
    }
    html! {
        (PreEscaped(format!(
            r#"<button type="submit" class="{}"{}>"#,
            escape_attr(&class),
            opts.attrs.as_string()
        )))
        @if let Some(name) = opts.icon_name {
            (icon(name, ""))
        }
        (opts.label)
        (PreEscaped("</button>"))
    }
}

pub struct ButtonLink<'a> {
    pub label: &'a str,
    pub href: &'a str,
    pub icon_name: Option<&'a str>,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonLink<'_> {
    fn default() -> Self {
        Self {
            label: "",
            href: "#",
            icon_name: None,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_link(opts: ButtonLink<'_>) -> Markup {
    let mut class = format!("btn {}", opts.classes);
    if opts.icon_name.is_some() && !opts.label.is_empty() {
        // Go ButtonLink uses flex (not inline-flex) when icon+label.
        class.push_str(" flex items-center gap-2");
    }
    let attrs = link_hx_attrs(opts.href).merge(&opts.attrs);
    html! {
        (PreEscaped(format!(
            r#"<a href="{}" class="{}"{}>"#,
            escape_attr(opts.href),
            escape_attr(&class),
            attrs.as_string()
        )))
        @if let Some(name) = opts.icon_name {
            (icon(name, ""))
        }
        (opts.label)
        (PreEscaped("</a>"))
    }
}

/// Typed app-pane navigation link for a route value.
pub fn button_link_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let href = route.url();
    button_link(ButtonLink {
        label,
        href: &href,
        classes,
        ..Default::default()
    })
}

/// Typed app-pane link with a pre-built URL (query strings).
pub fn button_link_url(href: &str, label: &str, classes: &str) -> Markup {
    button_link(ButtonLink {
        label,
        href,
        classes,
        ..Default::default()
    })
}

pub struct ButtonPost<'a> {
    pub label: &'a str,
    pub action: &'a str,
    pub icon_name: Option<&'a str>,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonPost<'_> {
    fn default() -> Self {
        Self {
            label: "",
            action: "",
            icon_name: None,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_post(opts: ButtonPost<'_>) -> Markup {
    let mut class = format!("btn {}", opts.classes);
    if opts.icon_name.is_some() && !opts.label.is_empty() {
        class.push_str(" inline-flex items-center gap-2");
    }
    let form_attrs = form_hx_boost_post_main(opts.action);
    // Go button_post.html: no class on the form wrapper.
    html! {
        (PreEscaped(format!(
            r##"<form{} @click.stop="">"##,
            form_attrs.as_string()
        )))
        (PreEscaped(format!(
            r#"<button type="submit" class="{}"{}>"#,
            escape_attr(&class),
            opts.attrs.as_string()
        )))
        @if let Some(name) = opts.icon_name {
            (icon(name, ""))
        }
        (opts.label)
        (PreEscaped("</button></form>"))
    }
}

/// Typed hx-boost POST button for redirect routes (e.g. logout).
pub fn button_post_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let action = route.path();
    button_post(ButtonPost {
        label,
        action: &action,
        classes,
        ..Default::default()
    })
}

pub struct ButtonClear<'a> {
    pub label: &'a str,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonClear<'_> {
    fn default() -> Self {
        Self {
            label: "Clear",
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_clear(opts: ButtonClear<'_>) -> Markup {
    let label = if opts.label.is_empty() {
        "Clear"
    } else {
        opts.label
    };
    let class = format!("btn btn-ghost my-2 {}", opts.classes);
    html! {
        (PreEscaped(format!(
            r#"<button type="button" class="{}"{} onclick="this.closest('form').querySelectorAll('input,select,textarea').forEach(el => {{ el.value = ''; }});">"#,
            escape_attr(&class),
            opts.attrs.as_string()
        )))
        (label)
        (PreEscaped("</button>"))
    }
}

pub struct ButtonDownload<'a> {
    pub label: &'a str,
    pub href: &'a str,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonDownload<'_> {
    fn default() -> Self {
        Self {
            label: "Download",
            href: "#",
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_download(opts: ButtonDownload<'_>) -> Markup {
    let attrs = opts
        .attrs
        .clone()
        .set("hx-boost", "false");
    html! {
        (PreEscaped(format!(
            r#"<a href="{}" download class="{}"{}>"#,
            escape_attr(opts.href),
            escape_attr(&format!("btn {}", opts.classes)),
            attrs.as_string()
        )))
        (opts.label)
        (PreEscaped("</a>"))
    }
}

/// Typed file-download link for a route value.
pub fn button_download_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let href = route.path();
    button_download(ButtonDownload {
        label,
        href: &href,
        classes,
        ..Default::default()
    })
}

use crate::components::htmx::{HTMX_SELECT_UNSET, HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL};
use crate::components::swap::SwapKey;

/// Button that GETs modal markup into `document.body` (Go `ButtonModal`).
pub struct ButtonModal<'a> {
    pub label: &'a str,
    pub href: &'a str,
    pub icon_name: Option<&'a str>,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonModal<'_> {
    fn default() -> Self {
        Self {
            label: "",
            href: "#",
            icon_name: None,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_modal(opts: ButtonModal<'_>) -> Markup {
    let mut class = format!("btn {}", opts.classes);
    if opts.icon_name.is_some() && !opts.label.is_empty() {
        class.push_str(" inline-flex items-center gap-2");
    }
    html! {
        div class="w-full fk-modal-host" {
            (PreEscaped(format!(
                r#"<button type="button" class="{}" hx-get="{}" hx-target="{}" hx-select="{}" hx-swap="{}" hx-push-url="false"{}>"#,
                escape_attr(&class),
                escape_attr(opts.href),
                HTMX_TARGET_BODY_MODAL,
                HTMX_SELECT_UNSET,
                HTMX_SWAP_BODY_MODAL,
                opts.attrs.as_string()
            )))
            @if let Some(name) = opts.icon_name {
                (icon(name, ""))
            }
            (opts.label)
            (PreEscaped("</button>"))
        }
    }
}

/// Typed modal opener for a modal GET route value.
pub fn button_modal_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let href = route.url();
    button_modal(ButtonModal {
        label,
        href: &href,
        classes,
        ..Default::default()
    })
}

/// Modal opener: GET loads a typed dialog into [`crate::components::ModalHostKey`].
///
/// The modal form itself must use declarative `hx-post` targeting the dialog key;
/// there is no custom event bus or `htmx.ajax` glue.
pub struct ButtonModalForm<'a> {
    pub label: &'a str,
    pub href: &'a str,
    pub name: &'a str,
    pub form_post_url: &'a str,
    pub modal_uid: &'a str,
    pub icon_name: Option<&'a str>,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for ButtonModalForm<'_> {
    fn default() -> Self {
        Self {
            label: "",
            href: "#",
            name: "",
            form_post_url: "",
            modal_uid: "",
            icon_name: None,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

pub fn button_modal_form(opts: ButtonModalForm<'_>) -> Markup {
    let mut href = opts.href.to_string();
    if !opts.name.is_empty() {
        let href_sep = if href.contains('?') { '&' } else { '?' };
        href = format!("{href}{href_sep}name={}", opts.name);
    }

    let mut class = format!("btn {}", opts.classes);
    if opts.icon_name.is_some() && !opts.label.is_empty() {
        class.push_str(" inline-flex items-center gap-2");
    }

    html! {
        div class="fk-modal-host" {
            (PreEscaped(format!(
                r#"<button type="button" class="{}" hx-get="{}" hx-target="{}" hx-select="{}" hx-swap="{}" hx-push-url="false"{}>"#,
                escape_attr(&class),
                escape_attr(&href),
                HTMX_TARGET_BODY_MODAL,
                HTMX_SELECT_UNSET,
                HTMX_SWAP_BODY_MODAL,
                opts.attrs.as_string()
            )))
            @if let Some(name) = opts.icon_name {
                (icon(name, ""))
            }
            (opts.label)
            (PreEscaped("</button>"))
        }
    }
}

/// Typed modal form opener for a modal GET route and POST path.
pub fn button_modal_form_route(
    get_route: impl RouteUrl,
    post_route: impl RouteUrl,
    label: &str,
    modal_uid: &str,
    classes: &str,
) -> Markup {
    let href = get_route.url();
    let post_url = post_route.path();
    button_modal_form(ButtonModalForm {
        label,
        href: &href,
        form_post_url: &post_url,
        modal_uid,
        classes,
        ..Default::default()
    })
}

/// Typed modal form opener with a pre-built GET href and POST path.
pub fn button_modal_form_urls(
    href: &str,
    form_post_url: &str,
    label: &str,
    modal_uid: &str,
    classes: &str,
) -> Markup {
    button_modal_form(ButtonModalForm {
        label,
        href,
        form_post_url,
        modal_uid,
        classes,
        ..Default::default()
    })
}

/// Like [`button_modal_form`] but appends `name` from a typed modal key's id for debugging.
pub fn button_modal_form_keyed<K: SwapKey>(mut opts: ButtonModalForm<'_>) -> Markup {
    if opts.modal_uid.is_empty() {
        opts.modal_uid = K::ID;
    }
    button_modal_form(opts)
}
