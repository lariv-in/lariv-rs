//! Progressive Web App manifest, service worker, and offline support.
//!
//! Injects web manifest links into the global HTML shell head and serves
//! static PWA resource routes. A topbar Install button captures
//! `beforeinstallprompt` and stays hidden when
//! `navigator.getInstalledRelatedApps()` reports a related app already installed.
//!
//! # Configurations
//!
//! - `[pwa]` → [`config::PwaConfig`]: app name, short name, theme color, icons, shortcuts, display
//!   override, static asset directories, service worker path, favicon path, and optional offline view name.
//!
//! # Shell chrome
//!
//! - App manifest path patched into [`shell_base`](crate::components::shell_base) from
//!   [`routes::PwaManifestRouteTag`] in [`StateHook`].
//! - Favicon path patched into the same shell from [`routes::PwaFaviconRouteTag`] when
//!   `faviconPath` is set.
//! - Apple iOS standalone tags (`apple-mobile-web-app-*`, `apple-touch-icon`, splash
//!   images) patched into the same shell from `[pwa]` config.
//! - Install-prompt script via [`slots::SlotsHook`].
//! - Topbar Install button (`#pwa-install`), hidden until the app is installable and not
//!   already installed.
//! - Document title patched from `PWA_APP_NAME` in [`StateHook`].
//!
//! # Routes
//!
//! - `/app.webmanifest` — JSON manifest from config (`id` + `related_applications` for
//!   `getInstalledRelatedApps`)
//! - `/favicon.ico` — favicon bytes from `faviconPath`
//! - `/serviceworker.js` — custom or default caching/offline service worker
//! - `/offline` — offline fallback page
//! - `/static/pwa/{*path}` — static PWA assets from `StaticDir`
//! - `/.well-known/assetlinks.json` — Android Digital Asset Links
//!
//! Set `offlineViewName` to a key on [`crate::views::ViewRegistry`] to serve a custom offline handler.

pub mod apple;
pub mod config;
pub mod handlers;
pub mod routes;
pub mod slots;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    components::{set_app_manifest_path, set_apple_pwa_head, set_document_title, set_favicon_path},
    config::{ConfigCap, ConfigTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{PwaConfig, PwaConfigTag};
use routes::{PwaFaviconRouteTag, PwaManifestRouteTag};

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

/// Copies loaded `[pwa]` config onto [`PwaTag`] and patches shell title, manifest, favicon, and Apple tags.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, CfgIdx, Configs, PwaCfgIdx, TagProof> AttachState<L, (CfgIdx, Configs, PwaCfgIdx, TagProof)>
    for StateHook
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
        set_app_manifest_path(PwaManifestRouteTag::PATH);
        if !config.favicon_path.is_empty() {
            set_favicon_path(PwaFaviconRouteTag::PATH);
        }
        set_apple_pwa_head(apple::apple_head(&config).into_string());
        app.add_capability(CapStore::with_items(config))
    }
}
