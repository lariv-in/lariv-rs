#![feature(impl_trait_in_assoc_type)]
//! CRM plugin — leads.

pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod lead_source;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod scope;
pub mod state;
pub mod templates;

pub mod apps;
pub mod create_modals;
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

lariv_core::define_plugin_install! {
    plugin: CrmTag;
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
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
