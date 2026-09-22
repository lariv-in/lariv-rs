//! MoQ video conferencing (rooms, embedded relay, uncomposited recordings).
//!
//! # Configurations
//!
//! - `[meets]` → [`config::MeetsConfig`]: room-code length, anonymous cookie secret.
//! - `[meets.transport]` → [`config::TransportConfig`]: MoQ relay bind, TLS certs, public URL.
//!
//! # Database models
//!
//! - [`entities::ConferenceRoom`]: string-code primary key, host, join flags.
//! - [`entities::JoinedUser`] / [`entities::AnonymousUser`]: membership.
//! - [`entities::MeetingRecording`]: file [`crate::plugins::filesystem::entities::VNode`] plus transcript.
//!
//! # Routes
//!
//! - `/meets/` — room list and create
//! - `/meets/{code}/` — lobby / in-call
//! - MoQ relay at `/meets/{code}` (embedded `moq-relay`, not Axum HTTP)

pub mod apps;
pub mod config;
pub mod cookies;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod moq_auth;
pub mod recording;
pub mod relay;
pub mod routes;
pub mod serve_startup;
pub mod state;
pub mod templates;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::AttachState,
    plugins::filesystem::{
        config::{FilesystemConfig, FilesystemConfigTag},
        storage::filestore_from_config,
    },
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{MeetsConfig, MeetsConfigTag};
use state::MeetsState;

/// Capability tag for the meets plugin.
pub struct MeetsTag;

define_passthrough_cap!(MeetsStateCap, MeetsTag, MeetsState);

define_plugin_install! {
    plugin: MeetsTag;
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(MeetsConfigTag, MeetsConfig),
        http(routes::Hook),
        state(StateHook),
        serve_startup(serve_startup::ServeStartupHook),
    ]
}

/// Attaches [`MeetsState`] (DB, filestore, MoQ relay config, auth keys).
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, MeetsCfgIdx, FsCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, MeetsCfgIdx, FsCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<MeetsConfigTag, MeetsCfgIdx, Value = MeetsConfig>,
    Configs: GetByTag<FilesystemConfigTag, FsCfgIdx, Value = FilesystemConfig>,
    L: HList + CapTagAbsent<MeetsTag, TagProof>,
{
    type Output = HCons<MeetsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let configs = &app.get_capability::<ConfigTag, CfgIdx>().items;
        let config =
            <Configs as GetByTag<MeetsConfigTag, MeetsCfgIdx>>::get_by_tag(configs).clone();
        let fs_config =
            <Configs as GetByTag<FilesystemConfigTag, FsCfgIdx>>::get_by_tag(configs).clone();
        let store = filestore_from_config(&fs_config);
        app.add_capability(CapStore::with_items(MeetsState::new(conn, store, config)))
    }
}
