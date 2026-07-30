//! Shell head injection: manifest link + document title (Go `pwa.manifestLink` / `pwaTitle`).
//!
//! Title patching is done in [`crate::hooks::AttachState`] for
//! [`crate::hooks::WithStateHook<super::PwaTag>`](crate::hooks::WithStateHook) via [`set_document_title`]: the
//! core [`CoreTitle`] head slot already reads that value, so install order does
//! not matter.

use maud::{Markup, html};

use crate::{
    capability::define_register_items,
    components::{
        HeadSlotTag, RegisterSlots, RenderSlot, SlotCapability, SlotCtx, SlotOf,
    },
    http::ProvideRequestCaps,
    template::{RegisterTemplates, TemplateCapability},
};

use super::PwaTag;

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
    trait: RegisterTemplates;
    method: register_templates;
    bounds: [Clone, ProvideRequestCaps, Send, Sync];
    items: [];
}

define_register_items! {
    plugin: PwaTag;
    capability: SlotCapability;
    trait: RegisterSlots;
    method: register_slots;
    wrapper: SlotOf;
    bounds: [];
    items: [
        ManifestIdx: PwaManifestLinkTag, HeadSlotTag => PwaManifestLink,
    ]
}
