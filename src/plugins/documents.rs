//! Documents plugin — typed identity documents, starting with Aadhaar cards.
//!
//! The UI lists and edits [`entities::document::Model`] only. `document_type_id`
//! is the primary key of the type-specific table (`aadhar_cards` for
//! [`document_type::DocumentType::AadharCard`]). Forms and detail pages load
//! that row; there is no Aadhaar screen. The assistant Rune environment can
//! create one through `create_document` and replace its fields through
//! `update_document`.

pub mod apps;
pub mod create_modals;
pub mod detail_actions;
pub mod document_type;
pub mod entities;
pub mod forms;
pub mod gender;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod preferences;
pub mod routes;
pub mod rune_env;
pub mod scope;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use crate::{
    app::App,
    capability::CapStore,
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    },
};

use state::DocumentsState;

/// Capability tag for the documents plugin.
pub struct DocumentsTag;

crate::define_passthrough_cap!(DocumentsStateCap, DocumentsTag, DocumentsState);

crate::define_plugin_install! {
    plugin: DocumentsTag;
    steps: [
        apps(apps::Hook),
        rune_env(rune_env::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`DocumentsState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<DocumentsTag, TagProof>,
{
    type Output = HCons<DocumentsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(DocumentsState::new(conn)))
    }
}
