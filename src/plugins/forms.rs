//! Survey-style forms with JSON question definitions and responses.
//!
//! Admin CRUD at `/forms`; responses are managed on each form's detail page.
//!
//! # Database models
//!
//! - [`entities::Form`]: titled form with a JSON list of [`types::FormQuestion`].
//! - [`entities::FormResponse`]: submitted answers keyed by [`types::FormQuestionId`].

pub mod apps;
pub mod components;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod rune_env;
pub mod state;
pub mod templates;
pub mod types;

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

use state::FormsState;

/// Capability tag for the forms plugin.
pub struct FormsTag;

crate::define_passthrough_cap!(FormsStateCap, FormsTag, FormsState);

crate::define_plugin_install! {
    plugin: FormsTag;
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

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<FormsTag, TagProof>,
{
    type Output = HCons<FormsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(FormsState::new(conn)))
    }
}
