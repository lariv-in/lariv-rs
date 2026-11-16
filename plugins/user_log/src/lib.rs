//! Audit log for one user, opened from that user's detail sidebar.
//!
//! Registers a "Logs" entry on the admin user-detail menu and serves
//! `/dashboard/users/u/{id}/logs`, the shared audit table locked to that user.

pub mod handlers;
pub mod menu;
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

use state::UserLogState;

/// Capability tag for the user log plugin state.
pub struct UserLogTag;

define_passthrough_cap!(UserLogStateCap, UserLogTag, UserLogState);

define_plugin_install! {
    plugin: UserLogTag;
    /// Register the per-user logs page, route, and sidebar link.
    steps: [
        cap_hook(lariv_plugin_users::user_detail_menu::UserDetailMenuTag, lariv_plugin_users::user_detail_menu::UserDetailMenuCap, menu::Hook),
        templates(templates::Hook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
    ]
}

/// Attaches [`UserLogState`] at app mount.
#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<UserLogTag, TagProof>,
{
    type Output = HCons<UserLogStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(UserLogState::new(conn)))
    }
}
