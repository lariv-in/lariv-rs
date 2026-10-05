#![feature(impl_trait_in_assoc_type)]
//! Virtual node (VNode) database filesystem.
//!
//! Integrates local or GCS blob storage with database VNode entities for
//! uploads, downloads, folder hierarchies, and file browsing.
//!
//! # Configurations
//!
//! - `[filesystem]` → [`config::FilesystemConfig`]: storage backend (local directory or GCS bucket),
//!   credentials, and path prefixes.
//!
//! # Database models
//!
//! - [`entities::VNode`]: file/directory nodes with parent links, sizes, and MIME metadata.
//!   [`storage::Filestore`] supports streaming writes via reader APIs.
//! - [`entities::FilesystemPreferences`]: owner, role, and access for the filesystem root.
//!
//! # Templates and layers
//!
//! - List, detail, create/update forms, and file selector ([`templates`]).
//! - View stacks in [`layers`] with auth via [`lariv_plugin_users::middleware::RequireAuth`].
//!
//! # Routes
//!
//! - `/filesystem/`, `/filesystem/create/`, `/filesystem/u/{id}/`, edit/delete
//! - `/filesystem/select/` — file picker table

pub mod apps;
pub mod config;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod layers;
pub mod migrations;
pub mod node;
pub mod permissions;
pub mod preferences;
pub mod routes;
pub mod state;
pub mod storage;
pub mod templates;
pub mod zip;

pub use apps::FILESYSTEM_APP_KEY;

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};

use lariv_core::plugin_install::define_plugin_install;
use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::config::{ConfigCap, ConfigTag};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    };

use config::{FilesystemConfig, FilesystemConfigTag};
use state::FilesystemState;
use storage::{DynFilestore, filestore_from_config};

/// Capability tag for the filesystem plugin state.
pub struct FilesystemTag;

define_passthrough_cap!(FilesystemStateCap, FilesystemTag, FilesystemState);

define_plugin_install! {
    plugin: FilesystemTag;
    /// Register filesystem deferred hooks (apps, migrations, templates, slots, config, roles, routes, state).
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(FilesystemConfigTag, FilesystemConfig),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`FilesystemState`] (DB, filestore, config) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, FsCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, FsCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<FilesystemConfigTag, FsCfgIdx, Value = FilesystemConfig>,
    L: HList + CapTagAbsent<FilesystemTag, TagProof>,
{
    type Output = HCons<FilesystemStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<FilesystemConfigTag, FsCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let store: Arc<DynFilestore> = filestore_from_config(&config);
        app.add_capability(CapStore::with_items(FilesystemState::new(
            conn, store, config,
        )))
    }
}
