//! HTTP response helpers for HTML pages, cookies, and HTMX.

mod htmx;

pub use htmx::{Htmx, HtmxRequestType, htmx_middleware, parse_element_id};

use axum::http::{HeaderValue, header};
use frunk::Generic;
use maud::Markup;

use crate::components::{FoldSlots, ShellChrome, SlotCapability, SlotCtx};
use crate::template::RenderTemplate;

/// Build a page from its `Generic` field HList and render it with `chrome`.
pub fn html_page<P: Generic + RenderTemplate>(fields: P::Repr, chrome: &ShellChrome) -> Markup {
    html_template(P::from(fields), chrome)
}

/// Render a Maud page to markup with folded slot chrome.
pub fn html_template<T: RenderTemplate>(template: T, chrome: &ShellChrome) -> Markup {
    template.render(chrome)
}

/// Fold request slots for `ctx`, then render the page.
pub fn html_page_with_slots<P, Slots>(
    fields: P::Repr,
    folder: &SlotCapability<Slots>,
    ctx: &SlotCtx,
) -> Markup
where
    P: Generic + RenderTemplate,
    Slots: FoldSlots,
{
    let chrome = folder.fold(ctx);
    html_page::<P>(fields, &chrome)
}

/// Render a full page, `#app-layout` pane, or `<main id="main-content">` for HTMX.
pub fn html_page_or_app_layout<P, Slots>(
    htmx: &Htmx,
    fields: P::Repr,
    folder: &SlotCapability<Slots>,
    ctx: &SlotCtx,
) -> Markup
where
    P: Generic + RenderTemplate + crate::template::RenderAppPane,
    Slots: FoldSlots,
{
    let page = P::from(fields);
    if htmx.wants_main_content() {
        return page.render_main();
    }
    if htmx.wants_app_layout() {
        return page.render_pane();
    }
    let chrome = folder.fold(ctx);
    page.render(&chrome)
}

/// See-other redirect helper.
pub fn redirect(path: &str) -> axum::response::Redirect {
    axum::response::Redirect::to(path)
}

pub use crate::layers::{html_built_page_or_app_layout, html_built_page_with_slots, render_from_data};

pub fn set_cookie_header(name: &str, value: &str, max_age_secs: i64, secure: bool) -> HeaderValue {
    let secure_flag = if secure { "; Secure" } else { "" };
    let raw = format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_secs}{secure_flag}"
    );
    HeaderValue::from_str(&raw).expect("cookie header")
}

pub fn clear_cookie_header(name: &str, secure: bool) -> HeaderValue {
    let secure_flag = if secure { "; Secure" } else { "" };
    let raw = format!("{name}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{secure_flag}");
    HeaderValue::from_str(&raw).expect("cookie header")
}

pub use header::SET_COOKIE;
