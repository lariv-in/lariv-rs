//! Shell head injection: manifest link + document title.
//!
//! Title patching is done in [`crate::hooks::AttachState`] for
//! [`super::StateHook`] via [`set_document_title`](crate::components::slots::set_document_title): the
//! core [`CoreTitle`](crate::components::slots::CoreTitle) head slot already reads that value, so install order does
//! not matter.

use maud::{Markup, html};

use crate::{
    capability::define_register_items,
    components::{
        HeadSlotTag, RenderSlot, SlotCapability, SlotRegistrar, SlotCtx, SlotOf,
    },
    http::ProvideRequestCaps,
    template::{TemplateCapability, TemplateRegistrar},
};


#[derive(Default)]
pub struct PwaManifestLink;

impl RenderSlot for PwaManifestLink {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        html! {
            link rel="manifest" href="/app.webmanifest";
        }
    }
}

// Asset endpoints are handlers, not HTML page templates.
define_register_items! {
    plugin: PwaTag;
    capability: TemplateCapability;
    trait: TemplateRegistrar;
    method: register_templates;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    items: [];
    hook: Hook;
}

define_register_items! {
    plugin: PwaTag;
    capability: SlotCapability;
    trait: SlotRegistrar;
    method: register_slots;
    wrapper: SlotOf;
    bounds: [];
    hook: SlotsHook;
    items: [
        ManifestIdx: PwaManifestLinkTag, HeadSlotTag => PwaManifestLink,
    ]
}
