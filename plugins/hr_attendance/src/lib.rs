//! HR attendance addon — punch in/out and attendance records patched onto the HR hub sidebar.

pub mod apps;
pub mod crumbs;
pub mod entities;
pub mod filters;
pub mod forms;
pub mod handlers;
pub mod hr_sidebar;
pub mod keys;
pub mod migrations;
pub mod routes;
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

use state::AttendanceState;

pub struct HrAttendanceTag;

lariv_core::define_passthrough_cap!(HrAttendanceStateCap, HrAttendanceTag, AttendanceState);

lariv_core::define_plugin_install! {
    plugin: HrAttendanceTag;
    steps: [
        cap_hook(lariv_plugin_hr::hr_sidebar::HrSidebarTag, lariv_plugin_hr::hr_sidebar::HrSidebarCap, hr_sidebar::Hook),
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
    L: HList + CapTagAbsent<HrAttendanceTag, TagProof>,
{
    type Output = HCons<HrAttendanceStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(AttendanceState::new(conn)))
    }
}
