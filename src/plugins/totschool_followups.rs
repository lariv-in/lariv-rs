//! Totschool followups addon — client follow-up letters (no app tile).

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

use config::{FollowupsGenaiConfig, FollowupsGenaiConfigTag};
use state::FollowupsState;
use crate::plugins::totschool_genai::followups::spawn_worker;

pub struct TotschoolFollowupsTag;

lariv_rs::define_passthrough_cap!(TotschoolFollowupsStateCap, TotschoolFollowupsTag, FollowupsState);

lariv_rs::define_plugin_install! {
    plugin: TotschoolFollowupsTag;
    steps: [
        export(export::ExportHook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(FollowupsGenaiConfigTag, FollowupsGenaiConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, FuCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, FuCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<frunk::HNil, Configs>>,
    Configs: GetByTag<FollowupsGenaiConfigTag, FuCfgIdx, Value = FollowupsGenaiConfig>,
    L: HList + CapTagAbsent<TotschoolFollowupsTag, TagProof>,
{
    type Output = HCons<TotschoolFollowupsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<FollowupsGenaiConfigTag, FuCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let worker = spawn_worker(config.api_key(), config.model.clone(), conn.clone());
        app.add_capability(CapStore::with_items(FollowupsState::new(conn, worker)))
    }
}
