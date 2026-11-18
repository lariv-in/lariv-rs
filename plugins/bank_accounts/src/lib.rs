//! Bank accounts plugin — account CRUD plus a synced IFSC branch directory.

pub mod apps;
pub mod entities;
pub mod forms;
pub mod handlers;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod scope;
pub mod serve_startup;
pub mod state;
pub mod sync;
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

use state::BankAccountsState;

pub struct BankAccountsTag;

lariv_core::define_passthrough_cap!(BankAccountsStateCap, BankAccountsTag, BankAccountsState);

lariv_core::define_plugin_install! {
    plugin: BankAccountsTag;
    steps: [
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
        apps(apps::Hook),
        serve_startup(serve_startup::ServeStartupHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<BankAccountsTag, TagProof>,
{
    type Output = HCons<BankAccountsStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(BankAccountsState::new(conn)))
    }
}
