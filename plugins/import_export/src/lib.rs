//! XLSX import and export for Lariv.
//!
//! Export downloads catalog tables as a workbook. Import reads that workbook
//! and upserts the same tables by primary key, preserving IDs and FK order.
//!
//! # Templates
//!
//! - [`templates::ImportPage`] — upload form and import result.
//! - [`templates::ExportPage`] — table selector and download.
//!
//! # Routes
//!
//! - `/import` — GET upload screen ([`handlers::import::page`]).
//! - `/import` — POST multipart workbook ([`handlers::import::import_post`]).
//! - `/export` — table selection screen ([`handlers::export::page`]).
//! - `/export/download` — POST download of selected models ([`handlers::export::download`]).
//!
//! Uploads share [`lariv_core::http::REQUEST_BODY_LIMIT_BYTES`] (50 MiB). Without that
//! layer, Axum's 2 MiB multipart default truncates the body mid-parse.

pub mod apps;
pub mod forms;
pub mod handlers;
pub mod routes;
pub mod state;
pub mod templates;
pub mod upsert;
pub mod xlsx;

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

use state::ImportExportState;

/// Plugin identity tag for import/export routes, templates, and state.
pub struct ImportExportPluginTag;

define_passthrough_cap!(
    ImportExportStateCap,
    ImportExportPluginTag,
    ImportExportState
);

define_plugin_install! {
    plugin: ImportExportPluginTag;
    steps: [
        apps(apps::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`ImportExportState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<ImportExportPluginTag, TagProof>,
{
    type Output = HCons<ImportExportStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(ImportExportState::new(conn)))
    }
}
