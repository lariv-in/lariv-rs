//! Users plugin — authentication, roles, and user administration.

pub mod apps;
pub mod auth;
pub mod cli;
pub mod config;
pub mod entities;
pub mod error;
pub mod handlers;
pub mod jwt;
pub mod keys;
pub mod layers;
pub mod middleware;
pub mod migrations;
pub mod password;
pub mod routes;
pub mod seed;
pub mod session;
pub mod state;
pub mod templates;

#[cfg(test)]
mod tests;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::{App, MountedApp},
    capability::{CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::{AttachState, RunSeed, SeedHook, WithStateHook},
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{UsersConfig, UsersConfigTag};
use state::UsersState;

/// Capability tag for the users plugin state.
pub struct UsersTag;

define_passthrough_cap!(UsersStateCap, UsersTag, UsersState);

define_plugin_install! {
    plugin: UsersTag;
    /// Register users deferred hooks and config section.
    steps: [
        apps,
        migrations,
        templates,
        slots,
        config(UsersConfigTag, UsersConfig),
        http,
        state,
        seeds,
        commands,
    ]
}

impl<L, DbIdx, CfgIdx, Configs, UsersCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, UsersCfgIdx, TagProof)> for WithStateHook<UsersTag>
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<UsersConfigTag, UsersCfgIdx, Value = UsersConfig>,
    L: HList + CapTagAbsent<UsersTag, TagProof>,
{
    type Output = HCons<UsersStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<UsersConfigTag, UsersCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        app.add_capability(CapStore::with_items(UsersState::new(conn, config)))
    }
}

#[async_trait::async_trait]
impl<M, UsersIdx> RunSeed<M, UsersIdx> for SeedHook<UsersTag>
where
    M: GetByTag<UsersTag, UsersIdx, Value = UsersState> + Sync,
{
    async fn run_seed(app: &MountedApp<M>) -> anyhow::Result<()> {
        seed::seed(app.get_capability_output::<UsersTag, UsersIdx>()).await?;
        Ok(())
    }
}
