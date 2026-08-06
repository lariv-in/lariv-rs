//! HTTP response helpers for HTML pages, cookies, and HTMX-aware rendering.
//!
//! Handlers typically build Maud [`Markup`] via [`html_page`], [`html_page_or_app_layout`],
//! or the layer-aware helpers [`html_built_page_or_app_layout`], [`html_built_page_with_slots`], and [`render_from_data`].
//!
//! # Routes
//!
//! Full-page responses use slot chrome from [`SharedChromeFolder`] or [`SlotCapability`].
//! HTMX partial responses branch on [`Htmx::wants_app_layout`] and [`Htmx::wants_main_content`].
//!
//! # Use cases
//!
//! - Render a Generic page struct with folded navigation chrome.
//! - Return `#app-layout` or `<main id="main-content">` fragments for HTMX swaps.
//! - Set session cookies or issue redirects compatible with HTMX.
//!
//! # Examples
//!
//! ```rust ignore
//! async fn user_list(
//!     htmx: Htmx,
//!     Cap(folder): Cap<SharedChromeFolder>,
//!     slot_ctx: SlotCtx,
//! ) -> Markup {
//!     html_page_or_app_layout::<UserListPage, _>(
//!         user_list_fields,
//!         &folder,
//!         &slot_ctx,
//!     )
//! }
//! ```

mod htmx;
mod modal_form;
mod query;

pub use htmx::{
    Htmx, HtmxRequestType, htmx_middleware, parse_element_id, respond_create_modal_done,
};
pub use modal_form::{ModalFormQuery, modal_create_post_url};
pub use query::{QueryI64, QueryPage, QueryStr, query_bool, query_i64, query_str, query_u32};

use axum::http::{HeaderValue, header};
use frunk::Generic;
use maud::Markup;

use crate::components::{ShellChrome, SlotCapability, SlotCtx};
use crate::template::RenderTemplate;

/// Build a page from its `Generic` field HList and render with slot chrome.
///
/// Prefer [`html_page_or_app_layout`] when the handler must support HTMX partial swaps.
pub fn html_page<P: Generic + RenderTemplate>(fields: P::Repr, chrome: &ShellChrome) -> Markup {
    html_template(P::from(fields), chrome)
}

/// Render a Maud page template with pre-folded slot chrome.
pub fn html_template<T: RenderTemplate>(template: T, chrome: &ShellChrome) -> Markup {
    template.render(chrome)
}

/// Fold request slots for `ctx`, then render the page (full document, no HTMX branching).
pub fn html_page_with_slots<P, Slots>(
    fields: P::Repr,
    folder: &SlotCapability<Slots>,
    ctx: &SlotCtx,
) -> Markup
where
    P: Generic + RenderTemplate,
    Slots: crate::components::FoldSlots,
{
    let chrome = folder.fold(ctx);
    html_page::<P>(fields, &chrome)
}

/// Render full page, `#app-layout` pane, or `<main id="main-content">` based on HTMX headers.
///
/// # Use cases
///
/// - Single handler serving both direct navigation and boosted/partial HTMX requests.
/// - Create/edit forms that POST back into `#app-layout`.
pub fn html_page_or_app_layout<P, Slots>(
    htmx: &Htmx,
    fields: P::Repr,
    folder: &SlotCapability<Slots>,
    ctx: &SlotCtx,
) -> Markup
where
    P: Generic + RenderTemplate + crate::template::RenderAppPane,
    Slots: crate::components::FoldSlots,
{
    let page = P::from(fields);
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    let chrome = folder.fold(ctx);
    page.render(&chrome)
}

pub use crate::layers::{html_built_page_or_app_layout, html_built_page_with_slots, render_from_data};

/// Build a `Set-Cookie` header for a session or preference cookie.
pub fn set_cookie_header(name: &str, value: &str, max_age_secs: i64, secure: bool) -> HeaderValue {
    let secure_flag = if secure { "; Secure" } else { "" };
    let raw = format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}{secure_flag}"
    );
    HeaderValue::from_str(&raw).expect("cookie header")
}

/// Build a `Set-Cookie` header that clears `name` (Max-Age=0).
pub fn clear_cookie_header(name: &str, secure: bool) -> HeaderValue {
    let secure_flag = if secure { "; Secure" } else { "" };
    let raw = format!("{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}");
    HeaderValue::from_str(&raw).expect("cookie header")
}

pub use header::SET_COOKIE;
