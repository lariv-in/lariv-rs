//! Website routing from database-backed page records.
//!
//! Maps URL paths to filesystem VNodes, renders minijinja/HTML pages,
//! and provides a GrapesJS visual builder for `.html`/`.htm`/`.tmpl` files.
//!
//! # Dynamic views security note
//!
//! Dynamic views cannot serve arbitrary files under a directory — that would allow arbitrary
//! read access via templates and the virtual filesystem.
//!
//! # Configurations
//!
//! - `[website]` → [`config::WebsiteConfig`]: `newPageRootDir` (blank page parent path),
//!   `assetsDir` (GrapesJS uploads; defaults to `{newPageRootDir}/assets`).
//!
//! # Database models
//!
//! - [`entities::DbRoute`]: active URL path → page VNode mapping; optional GrapesJS project JSON
//!   and theme registry key.
//!
//! # Templates and builder
//!
//! - Public catch-all [`handlers::dynamic`] and admin route CRUD ([`handlers::routes`], [`handlers::builder`]).
//! - GrapesJS blocks, components, traits, and themes registered via [`grapesjs::Hook`].
//!
//! # Routes
//!
//! - `/{path...}` — dynamic catch-all (patches home route)
//! - `/website/`, `/website/create/`, `/website/{id}/`, edit/delete
//! - `/website/{id}/builder/`, `/website/{id}/builder/project/`, `/website/{id}/builder/theme/`
//! - `/website/builder/assets/` — GrapesJS AssetManager upload
//! - `/media/{id}/` — public asset stream

pub mod apps;
pub mod builder;
pub mod builder_assets;
pub mod builder_refs;
pub mod config;
pub mod dotlottie;
pub mod entities;
pub mod forms;
pub mod grapesjs;
pub mod handlers;
pub mod html_edit;
pub mod keys;
pub mod match_route;
pub mod migrations;
pub mod publish;
pub mod render;
pub mod routes;
pub mod state;
pub mod template_funcs;
pub mod templates;
pub mod theme;

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::AttachState,
    plugins::filesystem::{
        config::{FilesystemConfig, FilesystemConfigTag, StorageBackend},
        storage::{DynFilestore, LocalFilestore, UnimplementedFilestore},
    },
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{WebsiteConfig, WebsiteConfigTag};
use state::WebsiteState;

/// Capability tag for the website plugin state.
pub struct WebsiteTag;

define_passthrough_cap!(WebsiteStateCap, WebsiteTag, WebsiteState);

define_plugin_install! {
    plugin: WebsiteTag;
    /// Register website deferred hooks (apps, grapesjs, migrations, templates, slots, config, routes, state).
    steps: [
        apps(apps::Hook),
        grapesjs(grapesjs::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(WebsiteConfigTag, WebsiteConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`WebsiteState`] (DB, filestore, website config) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, FsCfgIdx, WsCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, FsCfgIdx, WsCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<FilesystemConfigTag, FsCfgIdx, Value = FilesystemConfig>,
    Configs: GetByTag<WebsiteConfigTag, WsCfgIdx, Value = WebsiteConfig>,
    L: HList + CapTagAbsent<WebsiteTag, TagProof>,
{
    type Output = HCons<WebsiteStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let configs = &app.get_capability::<ConfigTag, CfgIdx>().items;
        let fs_config =
            <Configs as GetByTag<FilesystemConfigTag, FsCfgIdx>>::get_by_tag(configs).clone();
        let website_config =
            <Configs as GetByTag<WebsiteConfigTag, WsCfgIdx>>::get_by_tag(configs).clone();
        let store: Arc<DynFilestore> = match fs_config.storage_backend {
            StorageBackend::Local => Arc::new(LocalFilestore::new(fs_config.local_dir.clone())),
            StorageBackend::Gcs => Arc::new(UnimplementedFilestore),
        };
        app.add_capability(CapStore::with_items(WebsiteState::new(
            conn,
            store,
            website_config,
        )))
    }
}
