//! WebRTC SFU video conferencing (rooms, signaling, uncomposited recordings).
//!
//! # Configurations
//!
//! - `[meets]` → [`config::MeetsConfig`]: ICE/STUN/TURN, advertised IP, room-code length.
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
//! - `/meets/{code}/signal/` — JSON WebSocket signaling (not HTMX)

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
pub mod recording;
pub mod routes;
pub mod sfu;
pub mod signaling;
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
    ]
}

/// Attaches [`MeetsState`] (DB, filestore, ICE config, live SFU map).
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
