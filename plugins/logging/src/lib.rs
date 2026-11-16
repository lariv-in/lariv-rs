//! Audit log of inserts, updates, and deletes.
//!
//! Postgres triggers write one [`entities::Log`] row per statement on every public
//! table that has a single bigint primary key. [`session::install_session_hook`]
//! stamps `app.user_id` on each pooled connection so the trigger can record who
//! wrote the row. Call it before [`lariv_core::app::App::load_config`].

pub mod apps;
pub mod description;
pub mod entities;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod session;
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

use state::LoggingState;

/// Capability tag for the logging plugin.
pub struct LoggingTag;

define_passthrough_cap!(LoggingStateCap, LoggingTag, LoggingState);

define_plugin_install! {
    plugin: LoggingTag;
    /// Register the audit log tile, migrations, page, routes, and state.
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        http(routes::Hook),
        state(StateHook),
    ]
}

pub use session::install_session_hook;

/// Attaches [`LoggingState`] at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<LoggingTag, TagProof>,
{
    type Output = HCons<LoggingStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(LoggingState::new(conn)))
    }
}
