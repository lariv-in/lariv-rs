//! HR leaves addon — leave applications and leave-calc patched onto the HR hub.

pub mod apps;
pub mod create_modals;
pub mod crumbs;
pub mod entities;
pub mod filters;
pub mod forms;
pub mod handlers;
pub mod hr_sidebar;
pub mod keys;
pub mod logic;
pub mod migrations;
pub mod routes;
pub mod serve_startup;
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

use state::LeavesState;

// Ensure addon graph deps stay linked for leave_calc consumers.
#[allow(dead_code)]
fn _addon_graph_links() {
    let _ = std::any::type_name::<lariv_plugin_hr_attendance::HrAttendanceTag>();
    let _ = std::any::type_name::<lariv_plugin_hr_holidays::HrHolidaysTag>();
}

pub struct HrLeavesTag;

lariv_core::define_passthrough_cap!(HrLeavesStateCap, HrLeavesTag, LeavesState);

lariv_core::define_plugin_install! {
    plugin: HrLeavesTag;
    steps: [
        cap_hook(lariv_plugin_hr::hr_sidebar::HrSidebarTag, lariv_plugin_hr::hr_sidebar::HrSidebarCap, hr_sidebar::Hook),
        apps(apps::Hook),
        migrations(migrations::Hook),
        templates(templates::Hook),
        slots(templates::SlotsHook),
        cap_hook(lariv_plugin_users::role_authorization::RoleAuthorizationTag, lariv_plugin_users::role_authorization::RoleAuthorizationCap, routes::RoleHook),
        http(routes::Hook),
        state(StateHook),
        serve_startup(serve_startup::ServeStartupHook),
    ]
}

#[derive(Clone, Copy, Default)]
pub struct StateHook;

impl<L, DbIdx, TagProof> AttachState<L, (DbIdx, TagProof)> for StateHook
where
    L: GetByCapTag<DbTag, DbIdx, Value = DbCap>,
    L: HList + CapTagAbsent<HrLeavesTag, TagProof>,
{
    type Output = HCons<HrLeavesStateCap, L>;

    fn attach_state(app: App<L>) -> App<Self::Output> {
        let conn = app.get_capability::<DbTag, DbIdx>().items.conn.clone();
        app.add_capability(CapStore::with_items(LeavesState::new(conn)))
    }
}
