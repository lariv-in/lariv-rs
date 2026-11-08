#![feature(impl_trait_in_assoc_type)]
//! Point a filesystem directory at a git branch, tag, or commit and merge that tree.
//!
//! [`entities::FilesystemGitSync`] stores the remote URL, git identifier, and subpath for
//! one directory. The directory vnode is the primary key. Filesystem sidebars link to the
//! list at the root and to that directory's git sync page. **Git Sync** merges only when
//! the resolved commit differs from the last successful sync.

pub mod detail_action;
pub mod entities;
pub mod fetch;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod sidebar;
pub mod state;
pub mod sync_tree;
pub mod templates;
mod validate;

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::config::{ConfigCap, ConfigTag};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::plugin_install::define_plugin_install;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::{GetByCapTag, GetByTag},
};
use lariv_plugin_filesystem::{
    config::{FilesystemConfig, FilesystemConfigTag},
    storage::{DynFilestore, filestore_from_config},
};

use state::FilesystemSyncGitState;

/// Capability tag for the git directory sync plugin.
pub struct FilesystemSyncGitTag;

define_passthrough_cap!(
    FilesystemSyncGitStateCap,
    FilesystemSyncGitTag,
    FilesystemSyncGitState
);

define_plugin_install! {
    plugin: FilesystemSyncGitTag;
    /// Register git-sync config, the directory detail button, and the sync route.
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        cap_hook(lariv_plugin_filesystem::detail_actions::VNodeDetailActionsTag, lariv_plugin_filesystem::detail_actions::VNodeDetailActionsCap, detail_action::Hook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`FilesystemSyncGitState`] (DB and filestore) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, FsCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, FsCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<FilesystemConfigTag, FsCfgIdx, Value = FilesystemConfig>,
    L: HList + CapTagAbsent<FilesystemSyncGitTag, TagProof>,
{
    type Output = HCons<FilesystemSyncGitStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let fs_config = <Configs as GetByTag<FilesystemConfigTag, FsCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let store: Arc<DynFilestore> = filestore_from_config(&fs_config);
        app.add_capability(CapStore::with_items(FilesystemSyncGitState::new(
            conn, store,
        )))
    }
}
