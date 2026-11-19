//! Progressive Web App manifest, service worker, and offline support.
//!
//! Injects web manifest links into the global HTML shell head and serves
//! static PWA resource routes. A topbar Install button captures
//! `beforeinstallprompt` and stays hidden when
//! `navigator.getInstalledRelatedApps()` reports a related app already installed.
//!
//! # Configurations
//!
//! - `[pwa]` → [`config::PwaConfigSet`]: root app name, short name, theme color, icons,
//!   shortcuts, display override, static asset directories, favicon path, optional offline
//!   view name, plus optional `[[pwa.apps]]` extras selected by longest matching
//!   `PWA_APP_SCOPE`.
//!
//! # Shell chrome
//!
//! - App manifest path patched into [`shell_base`](lariv_core::components::shell_base) from
//!   [`routes::PwaManifestRouteTag`] in [`StateHook`] (path-scoped when extras exist).
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
//!   `getInstalledRelatedApps`); optional `?scope=` selects a path-scoped app
//! - `/favicon.ico` — favicon bytes from `faviconPath` (`?scope=` optional)
//! - `/serviceworker.js` — module worker that imports every registered `/sw/*.js` script
//! - `/sw/pwa.js` — default caching/offline service worker module
//! - `/offline` — offline fallback page (`?scope=` optional)
//! - `/static/pwa/{*path}` — static PWA assets from `StaticDir` (`?scope=` optional)
//! - `/.well-known/assetlinks.json` — Android Digital Asset Links (all configured apps)
//!
//! Set `offlineViewName` to a key on [`lariv_core::views::ViewRegistry`] to serve a custom offline handler.
//!
//! Service-worker modules are registered on [`ServiceWorkerRegistry`] with a route tag whose
//! `PATH` is under `/sw/`. The PWA plugin registers [`routes::PwaDefaultServiceWorkerRouteTag`]
//! itself. Another plugin adds a module with `cap_hook` and
//! [`ServiceWorkerModule`]`::<ItsRouteTag>::new(include_str!(...))`.

pub mod apple;
pub mod config;
pub mod handlers;
pub mod routes;
pub mod service_worker;
pub mod slots;
pub mod urls;

pub use service_worker::{
    ServiceWorkerCap, ServiceWorkerModule, ServiceWorkerRegistrar, ServiceWorkerRegistry,
    ServiceWorkerTag,
};

use frunk::{HCons, HNil, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::components::{
    register_apple_pwa_head_resolver, register_app_manifest_path_resolver,
    register_document_title_resolver, register_favicon_path_resolver, set_app_manifest_path,
    set_apple_pwa_head, set_document_title, set_favicon_path,
};
use lariv_core::config::{ConfigCap, ConfigTag};
use lariv_core::hooks::AttachState;
use lariv_core::plugin_install::define_plugin_install;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::{GetByCapTag, GetByTag},
};

use config::{PwaConfigSet, PwaConfigTag};
use routes::{PwaFaviconRouteTag, PwaManifestRouteTag};
use urls::{
    install_config_set, register_sw_scope_resolver, resolve_apple_head, resolve_document_title,
    resolve_favicon_path, resolve_manifest_path, resolve_sw_scope,
};

/// Capability tag for the PWA plugin (runtime config clone for [`lariv_core::http::Cap`] extraction).
pub struct PwaTag;

define_passthrough_cap!(PwaStateCap, PwaTag, PwaConfigSet);

define_plugin_install! {
    plugin: PwaTag;
    /// Register PWA config, head slot, and deferred route/state hooks.
    steps: [
        cap_attach(service_worker::ServiceWorkerTag, service_worker::ServiceWorkerCap, service_worker::ServiceWorkerCap::<frunk::HNil>::new()),
        cap_hook(service_worker::ServiceWorkerTag, service_worker::ServiceWorkerCap, service_worker::BaseHook),
        templates(slots::Hook),
        slots(slots::SlotsHook),
        config(PwaConfigTag, PwaConfigSet),
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
    Configs: GetByTag<PwaConfigTag, PwaCfgIdx, Value = PwaConfigSet>,
    L: HList + CapTagAbsent<PwaTag, TagProof>,
{
    type Output = HCons<PwaStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let loaded = <Configs as GetByTag<PwaConfigTag, PwaCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let config = install_config_set(loaded);
        let root = &config.root;

        if !root.app_name.is_empty() {
            set_document_title(root.app_name.clone());
        }
        set_app_manifest_path(PwaManifestRouteTag::PATH);
        if !root.favicon_path.is_empty() {
            set_favicon_path(PwaFaviconRouteTag::PATH);
        }
        set_apple_pwa_head(apple::apple_head(root).into_string());

        if config.has_extra_apps() {
            register_app_manifest_path_resolver(resolve_manifest_path);
            register_favicon_path_resolver(resolve_favicon_path);
            register_apple_pwa_head_resolver(resolve_apple_head);
            register_document_title_resolver(resolve_document_title);
            register_sw_scope_resolver(resolve_sw_scope);
        }

        app.add_capability(CapStore::with_items(config))
    }
}
