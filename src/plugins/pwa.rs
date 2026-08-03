//! Progressive Web App manifest, service worker, and offline support.
//!
//! Injects web manifest links into the global HTML shell head and serves
//! static PWA resource routes.
//!
//! # Configurations
//!
//! - `[pwa]` → [`config::PwaConfig`]: app name, theme color, icons, shortcuts, static asset
//!   directories, service worker path, and optional offline view name.
//!
//! # Shell head snippets
//!
//! - Manifest `<link rel="manifest">` injected via [`slots::SlotsHook`].
//! - Document title patched from `PWA_APP_NAME` in [`StateHook`].
//!
//! # Routes
//!
//! - `/app.webmanifest` — JSON manifest from config
//! - `/serviceworker.js` — custom or default caching/offline service worker
//! - `/offline` — offline fallback page
//! - `/static/pwa/{*path}` — static PWA assets from `StaticDir`
//! - `/.well-known/assetlinks.json` — Android Digital Asset Links
//!
//! Set `offlineViewName` to a key on [`crate::views::ViewRegistry`] to serve a custom offline handler.

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

/// Copies loaded `[pwa]` config onto [`PwaTag`] and applies the patch.
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
