//! XLSX data import for Lariv.
//!
//! Reads workbooks produced by the export plugin and upserts registered catalog
//! tables by primary key, preserving IDs and FK order.
//!
//! # Templates
//!
//! - [`templates::ImportPage`] — upload form and import result.
//!
//! # Routes
//!
//! - `/import` — GET upload screen ([`handlers::page`]).
//! - `/import` — POST multipart workbook ([`handlers::import_post`]).
//!
//! Uploads share [`crate::http::REQUEST_BODY_LIMIT_BYTES`] (50 MiB). Without that
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

use state::ImportState;

/// Plugin identity tag for import routes/templates/state.
pub struct ImportPluginTag;

define_passthrough_cap!(ImportStateCap, ImportPluginTag, ImportState);

define_plugin_install! {
    plugin: ImportPluginTag;
    steps: [
        apps(apps::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`ImportState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<ImportPluginTag, TagProof>,
{
    type Output = HCons<ImportStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(ImportState::new(conn)))
    }
}
