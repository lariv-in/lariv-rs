#![feature(impl_trait_in_assoc_type)]
//! Stored test cases for the LLM assistant.
//!
//! Each case has a name, a markdown definition (instructions for the assistant),
//! and a Rune solution script. `/dashboard/llm-assistant-tester` lists them.
//! Auth via [`lariv_plugin_users::middleware::RequireAuth`].

pub mod apps;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::{CapStore, define_passthrough_cap};
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::plugin_install::define_plugin_install;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::GetByCapTag,
};

use state::LlmAssistantTesterState;

/// Capability tag for the LLM assistant tester plugin.
pub struct LlmAssistantTesterTag;

define_passthrough_cap!(
    LlmAssistantTesterStateCap,
    LlmAssistantTesterTag,
    LlmAssistantTesterState
);

define_plugin_install! {
    plugin: LlmAssistantTesterTag;
    /// Register tester apps, migrations, templates, routes, and state.
    steps: [
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`LlmAssistantTesterState`] (DB connection) at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<LlmAssistantTesterTag, TagProof>,
{
    type Output = HCons<LlmAssistantTesterStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(LlmAssistantTesterState::new(conn)))
    }
}
