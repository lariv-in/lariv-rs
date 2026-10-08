#![feature(impl_trait_in_assoc_type)]
//! Finance credit notes plugin.

pub mod accounting_sidebar;
pub mod apps;
pub mod entities;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod scope;
pub mod source_docs;
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

use state::CreditnotesState;

pub struct FinanceCreditnotesTag;

lariv_core::define_passthrough_cap!(
    FinanceCreditnotesStateCap,
    FinanceCreditnotesTag,
    CreditnotesState
);

lariv_core::define_plugin_install! {
    plugin: FinanceCreditnotesTag;
    steps: [
        cap_hook(lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarTag, lariv_plugin_finance_accounts::accounting_sidebar::AccountingSidebarCap, accounting_sidebar::Hook),
        cap_hook(lariv_plugin_finance_accounts::SourceDocTag, lariv_plugin_finance_accounts::SourceDocCap, source_docs::Hook),
        apps(apps::Hook),
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
    L: HList + CapTagAbsent<FinanceCreditnotesTag, TagProof>,
{
    type Output = HCons<FinanceCreditnotesStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(CreditnotesState::new(conn)))
    }
}
