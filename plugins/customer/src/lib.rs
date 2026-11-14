//! Customer support addon — `customers` entity, create modal, and list table.
//!
//! Historical migrations stay registered so existing databases still recognize
//! applied versions. Live rows live in `customers` (created as `crm_customers`,
//! then renamed once the legacy address-shaped `customers` table is gone).
//! `m00010` finishes that rename when an empty `customers` blocked `m00009` /
//! finance `m00046` and a later `crm_customers` held the real rows.

pub mod create_modals;
pub mod entities;
pub mod foreign_keys;
pub mod forms;
pub mod gst_state;
pub mod handlers;
#[cfg(feature = "plugin-llm-assistant")]
pub mod hitl;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod rune_env;
pub mod state;
pub mod templates;

use frunk::{HCons, hlist::HList};

use lariv_core::app::App;
use lariv_core::capability::CapStore;
use lariv_core::db::{DbCap, DbTag};
use lariv_core::hooks::AttachState;
use lariv_core::traits::{
    add::{AddCapability, CapTagAbsent},
    get::GetByCapTag,
};

use state::CustomerState;

pub struct CustomerTag;

lariv_core::define_passthrough_cap!(CustomerStateCap, CustomerTag, CustomerState);

#[cfg(feature = "plugin-llm-assistant")]
lariv_core::define_plugin_install! {
    plugin: CustomerTag;
    steps: [
        cap_hook(lariv_plugin_llm_assistant::hitl::HitlTag, lariv_plugin_llm_assistant::hitl::HitlCap, hitl::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        rune_env(rune_env::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[cfg(not(feature = "plugin-llm-assistant"))]
lariv_core::define_plugin_install! {
    plugin: CustomerTag;
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        rune_env(rune_env::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<CustomerTag, TagProof>,
{
    type Output = HCons<CustomerStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(CustomerState::new(conn)))
    }
}
