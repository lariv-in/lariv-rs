//! Totschool appointments addon — appointment scheduling CRUD.

pub mod overlap;
pub mod ai;
pub mod config;
pub mod entities;
pub mod export;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use lariv_rs::{
    app::App,
    capability::CapStore,
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{AppointmentsGenaiConfig, AppointmentsGenaiConfigTag};
use state::AppointmentsState;
use crate::plugins::totschool_genai::appointments::spawn_worker;

pub struct TotschoolAppointmentsTag;

lariv_rs::define_passthrough_cap!(
    TotschoolAppointmentsStateCap,
    TotschoolAppointmentsTag,
    AppointmentsState
);

lariv_rs::define_plugin_install! {
    plugin: TotschoolAppointmentsTag;
    steps: [
        export(export::ExportHook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(AppointmentsGenaiConfigTag, AppointmentsGenaiConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, ApptCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, ApptCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<frunk::HNil, Configs>>,
    Configs: GetByTag<AppointmentsGenaiConfigTag, ApptCfgIdx, Value = AppointmentsGenaiConfig>,
    L: HList + CapTagAbsent<TotschoolAppointmentsTag, TagProof>,
{
    type Output = HCons<TotschoolAppointmentsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<AppointmentsGenaiConfigTag, ApptCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let worker = spawn_worker(config.api_key(), config.model.clone(), conn.clone());
        app.add_capability(CapStore::with_items(AppointmentsState::new(conn, worker)))
    }
}
