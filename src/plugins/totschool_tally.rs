//! Totschool tally plugin — daily progress tracking.

pub mod apps;
pub mod components;
pub mod entities;
pub mod export;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod session;
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

use state::TallyState;

pub struct TotschoolTallyTag;

lariv_rs::define_passthrough_cap!(TotschoolTallyStateCap, TotschoolTallyTag, TallyState);

lariv_rs::define_plugin_install! {
    plugin: TotschoolTallyTag;
    steps: [
        apps(apps::Hook),
        export(export::ExportHook),
        migrations(migrations::Hook),
        templates(templates::CombinedHook, UserDetailIdx),
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
    L: HList + CapTagAbsent<TotschoolTallyTag, TagProof>,
{
    type Output = HCons<TotschoolTallyStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(TallyState::new(conn)))
    }
}
