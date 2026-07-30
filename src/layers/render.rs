//! Terminal render helpers for pages built from layer Data.

use maud::Markup;

use crate::components::{FoldSlots, ShellChrome, SlotCapability, SlotCtx};
use crate::layers::BuildFromData;
use crate::template::{RenderAppPane, RenderTemplate};
use crate::web::Htmx;

/// Build `P` from layer data and render full page / app pane / main for HTMX.
pub fn render_from_data<P, Data, Slots>(
    htmx: &Htmx,
    data: &Data,
    slots: &SlotCapability<Slots>,
    slot_ctx: &SlotCtx,
) -> Markup
where
    P: BuildFromData<Data> + RenderTemplate + RenderAppPane,
    Slots: FoldSlots,
{
    let page = P::build_from_data(data);
    html_built_page_or_app_layout(&page, htmx, slots, slot_ctx)
}

/// Render an already-built page with HTMX granularity (no `Generic::Repr`).
pub fn html_built_page_or_app_layout<P, Slots>(
    page: &P,
    htmx: &Htmx,
    slots: &SlotCapability<Slots>,
    slot_ctx: &SlotCtx,
) -> Markup
where
    P: RenderTemplate + RenderAppPane,
    Slots: FoldSlots,
{
    if htmx.wants_main_content() {
        return page.render_main();
    }
    if htmx.wants_app_layout() {
        return page.render_pane();
    }
    let chrome = slots.fold(slot_ctx);
    page.render(&chrome)
}

/// Full-page render with slots (no HTMX pane branching).
pub fn html_built_page_with_slots<P, Slots>(
    page: &P,
    slots: &SlotCapability<Slots>,
    slot_ctx: &SlotCtx,
) -> Markup
where
    P: RenderTemplate,
    Slots: FoldSlots,
{
    let chrome: ShellChrome = slots.fold(slot_ctx);
    page.render(&chrome)
}
