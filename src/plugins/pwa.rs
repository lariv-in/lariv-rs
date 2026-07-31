//! Progressive Web App addon — manifest, service worker, offline page, static assets.
//!
//! Port of Go `p_pwa`: serves `/app.webmanifest`, `/serviceworker.js`, `/offline`,
//! `/static/pwa/{*path}`, and `/.well-known/assetlinks.json`, injects
//! `<link rel="manifest">`, and patches `core.Title` from `PWA_APP_NAME`.
//!
//! Set `offlineViewName` to a key registered on [`crate::views::ViewRegistry`]
//! (via `ViewRegistry::register` during plugin install) to serve that handler
//! for `/offline` instead of the default HTML.

pub mod config;
pub mod handlers;
pub mod routes;
pub mod slots;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    components::set_document_title,
    config::{ConfigCap, ConfigTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{PwaConfig, PwaConfigTag};

/// Capability tag for the PWA plugin (runtime config clone for [`crate::http::Cap`] extraction).
pub struct PwaTag;

define_passthrough_cap!(PwaStateCap, PwaTag, PwaConfig);

define_plugin_install! {
    plugin: PwaTag;
    /// Register PWA config, head slot, and deferred route/state hooks.
    steps: [
        templates(slots::Hook),
        slots(slots::SlotsHook),
        config(PwaConfigTag, PwaConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, CfgIdx, Configs, PwaCfgIdx, TagProof>
    AttachState<L, (CfgIdx, Configs, PwaCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<PwaConfigTag, PwaCfgIdx, Value = PwaConfig>,
    L: HList + CapTagAbsent<PwaTag, TagProof>,
{
    type Output = HCons<PwaStateCap, L>;

    /// Copy loaded `[p_pwa]` config onto [`PwaTag`] for request [`crate::http::Cap`] extraction.
    ///
    /// Also applies the Go `core.Title` patch: non-empty `PWA_APP_NAME` becomes the document title.
    fn attach_state(app: App<L>) -> App<Self::Output> {
        let config = <Configs as GetByTag<PwaConfigTag, PwaCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        if !config.app_name.is_empty() {
            set_document_title(config.app_name.clone());
        }
        app.add_capability(CapStore::with_items(config))
    }
}
