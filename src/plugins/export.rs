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

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    },
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
