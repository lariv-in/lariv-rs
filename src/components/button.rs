//! Action buttons for forms, navigation, downloads, and modals.
//!
//! Each builder returns [`maud::Markup`]. Prefer typed route helpers
//! (`button_link_route`, `button_post_route`, `button_modal_route`) so HTMX
//! attributes stay aligned with the typed swap keys.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::components::swap::{
    form_hx_boost_post_main, form_hx_post_redirect, hx_nav_app_layout_for_url,
};
use crate::components::text::icon;
use crate::http::{BoostPost, RouteUrl};

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

/// Primary form submit control.
///
/// Use inside a `<form>` for create/edit wizards and filter panels.
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

/// Render a styled submit button (optional icon).
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

/// Navigation link styled as a button.
///
/// Internal paths starting with `/` receive app-layout HTMX navigation attrs;
/// external URLs disable boost.
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

/// Render a link button with optional icon.
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

/// App-pane navigation link for a typed route value.
///
/// Use for toolbar actions and table create buttons that should swap `#app-layout`.
pub fn button_link_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let href = route.url();
    button_link(ButtonLink {
        label,
        href: &href,
        classes,
        ..Default::default()
    })
}

/// App-pane link with a pre-built URL (query strings, sort links).
pub fn button_link_url(href: &str, label: &str, classes: &str) -> Markup {
    button_link(ButtonLink {
        label,
        href,
        classes,
        ..Default::default()
    })
}

/// POST action wrapped in a one-field form with `hx-boost`.
///
/// Use for state-changing actions that redirect (logout, toggle) without a full page form.
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

/// Render a POST button inside an hx-boost form targeting main content.
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

/// hx-boost POST button for a typed route (e.g. logout).
pub fn button_post_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let action = route.path();
    button_post(ButtonPost {
        label,
        action: &action,
        classes,
        ..Default::default()
    })
}

/// Reset all inputs in the nearest ancestor form.
///
/// Use on filter/search forms to clear client-side field values.
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

/// Render a ghost "Clear" button that empties sibling inputs.
pub fn button_clear(opts: ButtonClear<'_>) -> Markup {
    let label = if opts.label.is_empty() {
        "Clear"
    } else {
        opts.label
    };
    let class = format!("btn btn-ghost {}", opts.classes);
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

/// Inline delete control for edit forms — POST with HTMX confirm, no separate delete page.
pub struct ButtonDeletePost<'a> {
    pub label: &'a str,
    pub confirm: &'a str,
    pub classes: &'a str,
}

impl Default for ButtonDeletePost<'_> {
    fn default() -> Self {
        Self {
            label: "Delete",
            confirm: "Permanently delete this item?",
            classes: "btn-error",
        }
    }
}

/// Render a small POST form with `hx-confirm` for destructive actions on edit pages.
pub fn button_delete_post_route<R: RouteUrl + BoostPost>(
    route: R,
    opts: ButtonDeletePost<'_>,
) -> Markup {
    let class = format!("btn {}", opts.classes);
    let attrs = form_hx_post_redirect(route).set("hx-confirm", opts.confirm);
    html! {
        (PreEscaped(format!(r#"<form method="POST"{}>"#, attrs.as_string())))
        button type="submit" class=(class) { (opts.label) }
        (PreEscaped("</form>"))
    }
}

/// Typed-route shorthand for [`button_delete_post_route`].
pub fn button_delete(route: impl RouteUrl + BoostPost, label: &str, confirm: &str) -> Markup {
    button_delete_post_route(
        route,
        ButtonDeletePost {
            label,
            confirm,
            ..Default::default()
        },
    )
}

/// File download link (`download` attribute, HTMX boost disabled).
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

/// Render a download link styled as a button.
pub fn button_download(opts: ButtonDownload<'_>) -> Markup {
    let attrs = opts.attrs.clone().set("hx-boost", "false");
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

/// File-download link for a typed route value.
pub fn button_download_route(route: impl RouteUrl, label: &str, classes: &str) -> Markup {
    let href = route.path();
    button_download(ButtonDownload {
        label,
        href: &href,
        classes,
        ..Default::default()
    })
}

use crate::components::htmx::{HTMX_SWAP_BODY_MODAL, HTMX_TARGET_BODY_MODAL};
use crate::components::swap::SwapKey;
use crate::web::{CreateModal, modal_create_get_for};

/// Button that GETs read-only modal markup into `document.body`.
///
/// Use for detail dialogs and pickers that do not submit a nested form.
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

/// Render an HTMX modal opener (`hx-get` → body, `outerHTML` swap).
pub fn button_modal(opts: ButtonModal<'_>) -> Markup {
    let mut class = format!("btn {}", opts.classes);
    if opts.icon_name.is_some() && !opts.label.is_empty() {
        class.push_str(" inline-flex items-center gap-2");
    }
    html! {
        div class="fk-modal-host" {
            (PreEscaped(format!(
                r#"<button type="button" class="{}" hx-get="{}" hx-target="{}" hx-swap="{}" hx-push-url="false"{}>"#,
                escape_attr(&class),
                escape_attr(opts.href),
                HTMX_TARGET_BODY_MODAL,
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

/// Modal opener for a typed GET route value.
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

/// Render a modal form opener; appends `name` query param when set.
///
/// On `htmx:config:request`, sets `refresh` to the closest `.data-table-container` id so
/// create handlers can close the modal and refresh the parent table. Avoids `hx-vals js:`
/// (brace-matching / JSON parse failures that surface as `Unexpected token '<'`).
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

    // Prefer closest `.data-table-container` refresh id; never strip a refresh already
    // baked into the URL (picker/list `table_create_button` embeds it). `this` is the
    // button (hx-on `thisArg`); `event.target` can be a child icon. HTMX 4 fires
    // `htmx:config:request` (`hx-on::config:request`); keep hyphenated aliases too.
    let refresh_js = concat!(
        "var t=this.closest('.data-table-container');",
        "var id=t?t.id:'';",
        "if(typeof ctx!=='undefined'&&ctx.request){",
        "var u=new URL(ctx.request.action,location.href);",
        "if(id){u.searchParams.set('refresh',id)}",
        "ctx.request.action=u.pathname+u.search+u.hash;",
        "if(id&&ctx.request.body&&ctx.request.body.set){ctx.request.body.set('refresh',id)}",
        "}else if(id){var p=event.detail.parameters;if(p&&p.set){p.set('refresh',id)}else if(p){p.refresh=id}}",
    );
    let refresh_on = format!(
        r#" hx-on::config:request="{js}" hx-on:htmx:config-request="{js}" hx-on:htmx:config:request="{js}""#,
        js = escape_attr(refresh_js),
    );
    let attrs = opts.attrs.as_string();

    html! {
        div class="fk-modal-host" {
            (PreEscaped(format!(
                r#"<button type="button" class="{}" hx-get="{}" hx-target="{}" hx-swap="{}" hx-push-url="false"{}{}>"#,
                escape_attr(&class),
                escape_attr(&href),
                HTMX_TARGET_BODY_MODAL,
                HTMX_SWAP_BODY_MODAL,
                refresh_on,
                attrs
            )))
            @if let Some(name) = opts.icon_name {
                (icon(name, ""))
            }
            (opts.label)
            (PreEscaped("</button>"))
        }
    }
}

/// Modal form opener for typed GET and POST routes.
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

/// Modal form opener with a pre-built GET href and POST path.
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

/// Plus-icon create control for a list or picker table; encodes parent refresh in the type system.
///
/// `T` is the [`.data-table-container`](crate::components::data_table) to refresh after create.
/// `M` is the create-modal swap key and its GET/POST routes ([`CreateModal`]).
///
/// `name` / `refresh` are already embedded in the GET URL by [`modal_create_get_for`], so they
/// must not be passed again through [`button_modal_form`] (which would append a duplicate
/// `name=` query param).
pub fn table_create_button<T: SwapKey, M: CreateModal>(
    icon_name: Option<&str>,
    classes: &str,
) -> Markup {
    let href = modal_create_get_for::<M, T>();
    button_modal_form(ButtonModalForm {
        name: "",
        href: &href,
        form_post_url: "",
        modal_uid: M::ID,
        icon_name,
        classes,
        ..Default::default()
    })
}
