#![feature(impl_trait_in_assoc_type)]
//! Serve an [mdBook](https://rust-lang.github.io/mdBook/) from the virtual filesystem.
//!
//! The book source (`book.toml` and `src/`) lives in the filesystem plugin.
//! [`publish`] exports that tree, builds it with [`mdbook_driver::MDBook`], writes
//! the HTML site back into the filesystem, and registers a website route for every
//! built file. Those routes use [`lariv_plugin_website::render::RAW_FILE_THEME`] so
//! the website plugin streams CSS, JavaScript, and HTML without running them
//! through minijinja. Relative links inside the book keep working because each
//! file is published at `{url prefix}/{path}`.
//!
//! # Preferences
//!
//! [`entities::MdBookPreferences`] is a singleton row edited at
//! `/dashboard/mdbook/preferences`: source directory, output directory, URL prefix,
//! and whether `serve` rebuilds the book. Source and output are foreign keys to
//! filesystem directories. A missing row is created with source directory
//! `mdbook`, output directory `mdbook-site`, prefix `/book`, and build-on-serve
//! enabled.
//!
//! # Commands
//!
//! - `mdbook-build` — rebuild and publish the book saved in preferences.
//!
//! On `serve`, the book is published when preferences say to build and the source
//! directory contains `SUMMARY.md`. A missing source directory, or one that is not
//! a book yet, is skipped.

pub mod apps;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod migrations;
pub mod paths;
pub mod preferences;
pub mod publish;
pub mod routes;
pub mod serve_startup;
pub mod state;
pub mod templates;

mod cli;

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

use state::MdBookState;

/// Capability tag for the mdBook plugin.
pub struct MdBookTag;

define_passthrough_cap!(MdBookStateCap, MdBookTag, MdBookState);

define_plugin_install! {
    plugin: MdBookTag;
    /// Register mdBook preferences, routes, the build command, and serve-time publishing.
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        commands(cli::Hook),
        serve_startup(serve_startup::ServeStartupHook),
        state(StateHook),
    ]
}

/// Attaches [`MdBookState`] (DB and filestore) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, FsCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, FsCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<FilesystemConfigTag, FsCfgIdx, Value = FilesystemConfig>,
    L: HList + CapTagAbsent<MdBookTag, TagProof>,
{
    type Output = HCons<MdBookStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let fs_config = <Configs as GetByTag<FilesystemConfigTag, FsCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let store: Arc<DynFilestore> = filestore_from_config(&fs_config);
        app.add_capability(CapStore::with_items(MdBookState::new(conn, store)))
    }
}

pub use publish::{PublishOutcome, publish};
