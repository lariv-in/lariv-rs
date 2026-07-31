//! Filesystem plugin — a simple database-backed virtual filesystem with local
//! (or GCS, unimplemented) blob storage.
//!
//! Port of Go `p_filesystem`: browse/create/edit/move/delete files and folders at
//! `/filesystem/…`, dashboard tile "Filesystem". View stacks live in [`layers`];
//! HTTP handlers seed auth via [`crate::plugins::users::middleware::RequireAuth`] and
//! run equivalent loader logic (`run_layers` inside `Route::get` currently hits rustc #100013).

pub mod apps;
pub mod config;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod layers;
pub mod migrations;
pub mod node;
pub mod routes;
pub mod state;
pub mod storage;
pub mod templates;
pub mod zip;

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{FilesystemConfig, FilesystemConfigTag, StorageBackend};
use state::FilesystemState;
use storage::{DynFilestore, LocalFilestore, UnimplementedFilestore};

/// Capability tag for the filesystem plugin state.
pub struct FilesystemTag;

define_passthrough_cap!(FilesystemStateCap, FilesystemTag, FilesystemState);

define_plugin_install! {
    plugin: FilesystemTag;
    /// Register filesystem deferred hooks (apps, migrations, templates, slots, config, routes, state).
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(FilesystemConfigTag, FilesystemConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

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
        let store: Arc<DynFilestore> = match config.storage_backend {
            StorageBackend::Local => Arc::new(LocalFilestore::new(config.local_dir.clone())),
            // GCS is not ported; every `Filestore` call fails at runtime instead of
            // panicking at startup (see `storage::UnimplementedFilestore`).
            StorageBackend::Gcs => Arc::new(UnimplementedFilestore),
        };
        app.add_capability(CapStore::with_items(FilesystemState::new(
            conn, store, config,
        )))
    }
}
