#![feature(impl_trait_in_assoc_type)]
//! Personal to-do items.
//!
//! Each row belongs to one user. Dashboard CRUD lives at `/dashboard/todo`.

pub mod apps;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::plugin_install::define_plugin_install;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::GetByCapTag,
};

use state::TodoState;

/// Capability tag for the todo plugin.
pub struct TodoTag;

define_passthrough_cap!(TodoStateCap, TodoTag, TodoState);

define_plugin_install! {
    plugin: TodoTag;
    /// Register todo apps, migrations, templates, routes, and state.
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`TodoState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<TodoTag, TagProof>,
{
    type Output = HCons<TodoStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(TodoState::new(conn)))
    }
}
