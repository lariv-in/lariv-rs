//! Excel spreadsheet (XLSX) creation and table export.
//!
//! Maps database metadata catalogs, tracks model relationship graphs,
//! and downloads filtered table records as XLSX workbooks.
//!
//! # Templates
//!
//! - [`templates::ExportPage`] — main export panel with table selector sidebar.
//!
//! # Routes
//!
//! - `/export/` — table selection screen ([`handlers::page`]).
//! - `/export/download/` — POST download of selected models ([`handlers::download`]).

pub mod apps;
pub mod handlers;
pub mod routes;
pub mod state;
pub mod templates;
pub mod xlsx;

use frunk::{HCons, hlist::HList};

use lariv_core::plugin_install::define_plugin_install;
use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    };

use state::ExportState;

/// Plugin identity tag for export routes/templates/state.
pub struct ExportPluginTag;

define_passthrough_cap!(ExportStateCap, ExportPluginTag, ExportState);

define_plugin_install! {
    plugin: ExportPluginTag;
    steps: [
        apps(apps::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`ExportState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<ExportPluginTag, TagProof>,
{
    type Output = HCons<ExportStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(ExportState::new(conn)))
    }
}
