//! Terminal render helpers — build pages from layer Data and render with HTMX branching.
//!
//! Called after [`run_layers`](crate::layers::run_layers) succeeds with `LayerStep::Continue`.
//! Combines [`BuildFromData`] with [`RenderTemplate`] / [`RenderAppPane`].
//!
//! # Use cases
//!
//! - Standard handler tail: run layers → `render_from_data` → return `Markup`.
//! - Render a pre-built page when layers only supply partial data.
//!
//! # Examples
//!
//! ```rust ignore
//! match run_layers(&view.layers, &mut ctx, &mut req).await {
//!     LayerStep::Done(res) => res,
//!     LayerStep::Continue(data) => {
//!         render_from_data::<UserEditPage, _>(&htmx, &data, &chrome, &slot_ctx).into_response()
//!     }
//! }
//! ```

use maud::Markup;

use crate::components::{SharedChromeFolder, SlotCtx};
use crate::layers::BuildFromData;
use crate::template::{RenderAppPane, RenderTemplate};
use crate::web::Htmx;

/// Build page `P` from folded layer `data`, then render full page / app pane / main for HTMX.
pub fn render_from_data<P, Data>(
    htmx: &Htmx,
    data: &Data,
    chrome: &SharedChromeFolder,
    slot_ctx: &SlotCtx,
) -> Markup
where
    P: BuildFromData<Data> + RenderTemplate + RenderAppPane,
{
    let page = P::build_from_data(data);
    html_built_page_or_app_layout(&page, htmx, chrome, slot_ctx)
}

/// Render an already-built page with HTMX granularity (no `Generic::Repr` / `BuildFromData`).
pub fn html_built_page_or_app_layout<P>(
    page: &P,
    htmx: &Htmx,
    chrome: &SharedChromeFolder,
    slot_ctx: &SlotCtx,
) -> Markup
where
    P: RenderTemplate + RenderAppPane,
{
    if htmx.wants_main_content() {
        return page.render_main();
    }
    if htmx.wants_app_layout() {
        return page.render_pane();
    }
    let shell = chrome.fold(slot_ctx);
    page.render(&shell)
}

/// Full-page render with slot chrome (no HTMX pane/main branching).
pub fn html_built_page_with_slots<P>(
    page: &P,
    chrome: &SharedChromeFolder,
    slot_ctx: &SlotCtx,
) -> Markup
where
    P: RenderTemplate,
{
    let shell = chrome.fold(slot_ctx);
    page.render(&shell)
}
