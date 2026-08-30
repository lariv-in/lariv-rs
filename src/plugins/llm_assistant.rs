//! Interactive LLM chat assistant powered by Gemini.
//!
//! Supports chat history, custom tool calling (trigram DB search, web search,
//! webpage fetch, file read, Rune execution), user prompt templates (skills),
//! and WebSocket streaming chat.
//! Gemini client lives in [`genai`] (no separate `p_google_genai` plugin).
//!
//! # Configurations
//!
//! - `[llm_assistant]` → [`config::LlmAssistantConfig`]: default chat model (used until preferences set one).
//! - Gemini API key, selected model, and Google CSE credentials (`cseApiKey`, `cseCx`) live in DB
//!   preferences ([`preferences`]); edit via `/llm-assistant/preferences`.
//!
//! # Database models
//!
//! - [`entities::Session`]: conversation thread with user reference.
//! - [`entities::SessionMessage`] / part entities: message contents, roles, tool calls/responses.
//! - [`entities::Skill`]: custom prompt templates / system instructions.
//! - [`entities::LlmAssistantPreferences`]: Gemini API key, model, CSE credentials, and related settings.
//!
//! # Templates
//!
//! Chat UI, session history list, skills management, and preferences pages (see [`templates`]).
//!
//! # Routes
//!
//! - `/llm-assistant/` — main chat view
//! - `/llm-assistant/history/` — previous sessions
//! - `/llm-assistant/skills/` — skill CRUD
//! - `/llm-assistant/preferences/` — Gemini API key, model, CSE credentials, and assistant settings
//! - `/llm-assistant/ws/` — WebSocket streaming endpoint

pub mod actions;
pub mod apps;
pub mod chat_attachments;
pub mod config;
pub mod content;
pub mod email_attachments;
pub mod email_listener;
pub mod email_mime;
pub mod email_pipeline;
pub mod email_send;
pub mod entities;
pub mod forms;
pub mod genai;
pub mod handlers;
pub mod keys;
pub mod live_turn;
pub mod migrations;
pub mod preferences;
pub mod routes;
pub mod rune_engine;
pub mod skill_hints;
pub mod skill_zip;
pub mod serve_startup;
pub mod state;
pub mod templates;
pub mod tools;
pub mod ws;

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};

use crate::plugin_install::define_plugin_install;
use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, define_passthrough_cap},
    config::{ConfigCap, ConfigTag},
    db::{DbCap, DbTag},
    hooks::AttachState,
    llm_tools::{LlmToolsCap, LlmToolsCapability, LlmToolsTag},
    plugins::filesystem::{
        config::{FilesystemConfig, FilesystemConfigTag},
        storage::{DynFilestore, filestore_from_config},
    },
    rune_env::{RuneEnvCap, RuneEnvCapability, RuneEnvTag},
    traits::{
        add::{AddCapability, CapTagAbsent},
        get::{GetByCapTag, GetByTag},
    },
};

use config::{LlmAssistantConfig, LlmAssistantConfigTag};
use state::{EmailAutomationDeps, LlmAssistantState};

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
        serve_startup(serve_startup::ServeStartupHook),
    ]
}

/// Attaches [`LlmAssistantState`] (DB + assistant config) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<
        L,
        DbIdx,
        CfgIdx,
        Configs,
        AsstCfgIdx,
        FsCfgIdx,
        ToolsIdx,
        ToolsHooks,
        ToolsProof,
        RuneEnvIdx,
        RuneEnvHooks,
        RuneProof,
        TagProof,
    > AttachState<L, (DbIdx, CfgIdx, Configs, AsstCfgIdx, FsCfgIdx, ToolsIdx, ToolsProof, RuneEnvIdx, RuneProof, TagProof)>
    for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: GetByCapTag<ConfigTag, CfgIdx, Value = ConfigCap<HNil, Configs>>,
    Configs: GetByTag<LlmAssistantConfigTag, AsstCfgIdx, Value = LlmAssistantConfig>,
    Configs: GetByTag<FilesystemConfigTag, FsCfgIdx, Value = FilesystemConfig>,
    L: GetByCapTag<LlmToolsTag, ToolsIdx, Value = LlmToolsCap<ToolsHooks>>,
    ToolsHooks: Clone + ApplyHooks<LlmToolsCapability, ToolsProof, Output = LlmToolsCapability>,
    L: GetByCapTag<RuneEnvTag, RuneEnvIdx, Value = RuneEnvCap<RuneEnvHooks>>,
    RuneEnvHooks: Clone + ApplyHooks<RuneEnvCapability, RuneProof, Output = RuneEnvCapability>,
    L: HList + CapTagAbsent<LlmAssistantTag, TagProof>,
{
    type Output = HCons<LlmAssistantStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        let configs = &app.get_capability::<ConfigTag, CfgIdx>().items;
        let config =
            <Configs as GetByTag<LlmAssistantConfigTag, AsstCfgIdx>>::get_by_tag(configs).clone();
        let fs_config =
            <Configs as GetByTag<FilesystemConfigTag, FsCfgIdx>>::get_by_tag(configs).clone();
        let store: Arc<DynFilestore> = filestore_from_config(&fs_config);
        let tools_cap = app.get_capability::<LlmToolsTag, ToolsIdx>();
        let tools = Arc::new(
            tools_cap
                .hooks
                .clone()
                .apply_hooks(tools_cap.items.clone()),
        );
        let rune_cap = app.get_capability::<RuneEnvTag, RuneEnvIdx>();
        let rune_env = Arc::new(
            rune_cap
                .hooks
                .clone()
                .apply_hooks(rune_cap.items.clone()),
        );
        let email_automation = EmailAutomationDeps {
            store,
            tools,
            rune_env,
        };
        let state = LlmAssistantState::new(conn, config, email_automation).bind_email_listener();
        app.add_capability(CapStore::with_items(state))
    }
}
