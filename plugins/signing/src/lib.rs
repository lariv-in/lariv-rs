#![feature(impl_trait_in_assoc_type)]
//! Per-user PDF signing.
//!
//! Each user has at most one [`entities::user_signature::Model`]. The private
//! key lives in the configured backend (a local directory for debugging, or
//! Cloud KMS). Reads always filter to the logged-in user, including superuser.

pub mod backend;
pub mod cert;
pub mod config;
pub mod documents_action;
pub mod entities;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod pdf;
pub mod routes;
pub mod scope;
pub mod state;
pub mod templates;

use frunk::{HCons, HNil, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::config::{ConfigCap, ConfigTag};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    };

use backend::key_backend_from_config;
use config::{SigningConfig, SigningConfigTag};
use state::SigningState;

/// Capability tag for the signing plugin.
pub struct SigningTag;

define_passthrough_cap!(SigningStateCap, SigningTag, SigningState);

lariv_core::define_plugin_install! {
    plugin: SigningTag;
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        config(SigningConfigTag, SigningConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`SigningState`] and registers the Documents sign button.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, SignCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, SignCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<SigningConfigTag, SignCfgIdx, Value = SigningConfig>,
    L: HList + CapTagAbsent<SigningTag, TagProof>,
{
    type Output = HCons<SigningStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        documents_action::register();
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<SigningConfigTag, SignCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        let keys = key_backend_from_config(&config);
        app.add_capability(CapStore::with_items(SigningState::new(conn, keys)))
    }
}
