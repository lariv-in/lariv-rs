//! CRM plugin — leads, companies, and contacts.

pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod lead_source;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod scope;
pub mod state;
pub mod templates;

pub mod apps;
pub mod create_modals;
pub mod crumbs;
pub mod detail_menu;

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

use state::CrmState;

pub struct CrmTag;

crate::define_passthrough_cap!(
    CrmStateCap,
    CrmTag,
    CrmState
);

crate::define_plugin_install! {
    plugin: CrmTag;
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
        apps(apps::Hook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<CrmTag, TagProof>,
{
    type Output = HCons<CrmStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(CrmState::new(conn)))
    }
}
