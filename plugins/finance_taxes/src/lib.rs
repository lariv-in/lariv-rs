#![feature(impl_trait_in_assoc_type)]
//! Finance taxes plugin.

pub mod accounting_sidebar;
pub mod apps;
pub mod create_modals;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod migrations;
pub mod routes;
pub mod rune_env;
pub mod scope;
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

use state::TaxesState;

pub struct FinanceTaxesTag;

lariv_core::define_passthrough_cap!(FinanceTaxesStateCap, FinanceTaxesTag, TaxesState);

lariv_core::define_plugin_install! {
    plugin: FinanceTaxesTag;
    steps: [
        cap_hook(lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarTag, lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        apps(apps::Hook),
        rune_env(rune_env::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
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
    L: HList + CapTagAbsent<FinanceTaxesTag, TagProof>,
{
    type Output = HCons<FinanceTaxesStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(TaxesState::new(conn)))
    }
}
