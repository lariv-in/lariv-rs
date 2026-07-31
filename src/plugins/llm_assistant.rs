//! LLM Assistant plugin — Gemini chat, skills, and session history.
//!
//! Port of Go `p_llm_assistant`. Gemini client lives in [`genai`] (no separate `p_google_genai`).

pub mod actions;
pub mod apps;
pub mod config;
pub mod content;
pub mod entities;
pub mod forms;
pub mod genai;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod rune_engine;
pub mod skill_zip;
pub mod state;
pub mod templates;
pub mod tools;
pub mod ws;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::AttachState,
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{LlmAssistantConfig, LlmAssistantConfigTag};
use state::LlmAssistantState;

/// Capability tag for the LLM assistant plugin state.
pub struct LlmAssistantTag;

define_passthrough_cap!(LlmAssistantStateCap, LlmAssistantTag, LlmAssistantState);

define_plugin_install! {
    plugin: LlmAssistantTag;
    /// Register assistant deferred hooks (apps, tools, migrations, templates, slots, config, routes, state).
    steps: [
        apps(apps::Hook),
        tools(tools::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        config(LlmAssistantConfigTag, LlmAssistantConfig),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, CfgIdx, Configs, AsstCfgIdx, TagProof>
    AttachState<L, (DbIdx, CfgIdx, Configs, AsstCfgIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<LlmAssistantConfigTag, AsstCfgIdx, Value = LlmAssistantConfig>,
    L: HList + CapTagAbsent<LlmAssistantTag, TagProof>,
{
    type Output = HCons<LlmAssistantStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let config = <Configs as GetByTag<LlmAssistantConfigTag, AsstCfgIdx>>::get_by_tag(
            &app.get_capability::<ConfigTag, CfgIdx>().items,
        )
        .clone();
        app.add_capability(CapStore::with_items(LlmAssistantState::new(conn, config)))
    }
}
