//! CRM plugin — leads.

pub mod entities;
pub mod forms;
pub mod handlers;
#[cfg(feature = "plugin-llm-assistant")]
pub mod hitl;
pub mod keys;
pub mod lead_source;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod rune_env;
pub mod scope;
pub mod state;
pub mod templates;

mod customer_ui;

pub mod apps;
pub mod crumbs;
pub mod detail_menu;

use frunk::{HCons, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::CapStore;
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::GetByCapTag,
};

use state::CrmState;

pub struct CrmTag;

lariv_core::define_passthrough_cap!(CrmStateCap, CrmTag, CrmState);

#[cfg(feature = "plugin-llm-assistant")]
lariv_core::define_plugin_install! {
    plugin: CrmTag;
    steps: [
        cap_hook(lariv_plugin_llm_assistant::hitl::HitlTag, lariv_plugin_llm_assistant::hitl::HitlCap, hitl::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        rune_env(rune_env::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
        apps(apps::Hook),
    ]
}

#[cfg(not(feature = "plugin-llm-assistant"))]
lariv_core::define_plugin_install! {
    plugin: CrmTag;
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        rune_env(rune_env::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
        apps(apps::Hook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<CrmTag, TagProof>,
{
    type Output = HCons<CrmStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(CrmState::new(conn)))
    }
}
