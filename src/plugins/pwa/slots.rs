//! Shell chrome: manifest link, install-prompt script, and topbar Install button.
//!
//! Title patching is done in [`crate::hooks::AttachState`] for
//! [`super::StateHook`] via [`set_document_title`](crate::components::slots::set_document_title): the
//! core [`CoreTitle`](crate::components::slots::CoreTitle) head slot already reads that value, so install order does
//! not matter.
//!
//! The Install button stays hidden until Chromium fires `beforeinstallprompt` and
//! [`navigator.getInstalledRelatedApps`](https://developer.mozilla.org/en-US/docs/Web/API/Navigator/getInstalledRelatedApps)
//! reports no related apps (see the `related_applications` member of `/app.webmanifest`).

use maud::{Markup, PreEscaped, html};

use crate::{
    capability::define_register_items,
    components::{
        HeadSlotTag, RenderSlot, SlotCapability, SlotCtx, SlotOf, SlotRegistrar,
        TopbarItemsSlotTag, icon,
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

#[derive(Default)]
pub struct PwaInstallScript;

impl RenderSlot for PwaInstallScript {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        // Closed <script> so the rest of the document is not treated as JS.
        html! {
            (PreEscaped(concat!(
                "<script>",
                include_str!("install.js"),
                "</script>"
            )))
        }
    }
}

#[derive(Default)]
pub struct PwaInstallButton;

impl RenderSlot for PwaInstallButton {
    fn render_slot(&self, _ctx: &SlotCtx) -> Markup {
        html! {
            (PreEscaped(
                r##"<button type="button" id="pwa-install" class="btn items-center btn-sm btn-square btn-outline" hidden aria-label="Install app">"##,
            ))
            (icon("arrow-down-tray", ""))
            (PreEscaped("</button>"))
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
        InstallScriptIdx: PwaInstallScriptTag, HeadSlotTag => PwaInstallScript,
        InstallBtnIdx: PwaInstallButtonTag, TopbarItemsSlotTag => PwaInstallButton,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markup_str(m: Markup) -> String {
        m.into_string()
    }

    #[test]
    fn pwa_slots_inject_manifest_install_script_and_hidden_button() {
        let slots = SlotCapability::new()
            .add::<PwaManifestLinkTag, HeadSlotTag, PwaManifestLink>()
            .add::<PwaInstallScriptTag, HeadSlotTag, PwaInstallScript>()
            .add::<PwaInstallButtonTag, TopbarItemsSlotTag, PwaInstallButton>();
        let chrome = slots.fold_chrome(&SlotCtx::default());

        let head = markup_str(chrome.head);
        assert!(head.contains(r#"rel="manifest""#));
        assert!(head.contains("/app.webmanifest"));
        assert!(head.contains("navigator.serviceWorker.register(\"/serviceworker.js\")"));
        assert!(head.contains("beforeinstallprompt"));
        assert!(head.contains("getInstalledRelatedApps"));
        assert!(
            head.contains("</script>"),
            "inline install script must be closed: {head}"
        );

        let topbar = markup_str(chrome.topbar_items);
        assert!(topbar.contains(r#"id="pwa-install""#));
        assert!(topbar.contains("hidden"));
        assert!(topbar.contains("arrow-down-tray") || topbar.contains("Install app"));
        assert!(topbar.contains(r#"aria-label="Install app""#));
    }
}
