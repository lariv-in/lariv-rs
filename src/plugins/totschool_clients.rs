//! Totschool clients plugin — hub app for client CRM.

pub mod apps;
pub mod entities;
pub mod export;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod related;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use lariv_rs::{
    app::App,
    capability::CapStore,
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::GetByCapTag,
    },
};

use state::ClientsState;

pub struct TotschoolClientsTag;

lariv_rs::define_passthrough_cap!(TotschoolClientsStateCap, TotschoolClientsTag, ClientsState);

lariv_rs::define_plugin_install! {
    plugin: TotschoolClientsTag;
    steps: [
        apps(apps::Hook),
        export(export::ExportHook),
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
    L: HList + CapTagAbsent<TotschoolClientsTag, TagProof>,
{
    type Output = HCons<TotschoolClientsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(ClientsState::new(conn)))
    }
}
