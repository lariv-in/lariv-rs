//! HR plugin — applicant lifecycle through employment.

pub mod apps;
pub mod create_modals;
pub mod crumbs;
pub mod detail_menu;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod public_page;
pub mod roles;
pub mod routes;
pub mod scope;
pub mod seed;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use crate::{
    app::{App, MountedApp},
    capability::CapStore,
    db::{DbCap, DbTag},
    hooks::{AttachState, RunSeed},
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use state::HrState;

pub struct HrTag;

crate::define_passthrough_cap!(HrStateCap, HrTag, HrState);

crate::define_plugin_install! {
    plugin: HrTag;
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
        seeds(SeedsHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<HrTag, TagProof>,
{
    type Output = HCons<HrStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(HrState::new(conn)))
    }
}

#[derive(Clone, Copy, Default)]
pub struct SeedsHook;

#[async_trait::async_trait]
impl<M, HrIdx> RunSeed<M, HrIdx> for SeedsHook
where
    M: GetByTag<HrTag, HrIdx, Value = HrState> + Sync,
{
    async fn run_seed(app: &MountedApp<M>) -> anyhow::Result<()> {
        seed::seed(&app.get_capability_output::<HrTag, HrIdx>().db).await?;
        Ok(())
    }
}
