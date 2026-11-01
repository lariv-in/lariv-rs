//! Overtime links patched onto the shared HR app sidebar.

use lariv_plugin_hr::hr_sidebar::{HrSidebarRegistrar, HrSidebarRegistry, link_with_prefixes};
use lariv_plugin_users::role_authorization::{current_auth, roles_for};
use lariv_plugin_users::roles::Superuser;

use crate::routes::{
    OvertimeApplicationsRouteTag, OvertimeApprovalsRouteTag, OvertimeListRouteTag, OvertimeView,
};

const OVERTIME_MATCH_PREFIXES: &[&str] = &[OvertimeListRouteTag::PATH];
const OVERTIME_QUEUE_MATCH_PREFIXES: &[&str] = &[OvertimeApprovalsRouteTag::PATH];

/// True when the signed-in user is not a superuser (overtime uses the self-service pages).
fn self_service() -> bool {
    current_auth().is_some_and(|auth| !Superuser::matches(&auth.role))
}

fn overtime_url() -> String {
    if self_service() {
        OvertimeApplicationsRouteTag.url()
    } else {
        OvertimeListRouteTag.url()
    }
}

fn manager_queue_visible() -> bool {
    self_service() && lariv_plugin_hr::nav::leave_approvals_visible()
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HrSidebarRegistrar for Hook {
    fn register_hr_sidebar(self, cap: HrSidebarRegistry) -> HrSidebarRegistry {
        cap.push(
            link_with_prefixes::<OvertimeListRouteTag>(
                "overtime",
                "Overtime",
                60,
                None,
                OVERTIME_MATCH_PREFIXES,
            )
            .with_url_resolver(overtime_url)
            .with_roles(roles_for::<OvertimeView>),
        )
        .push(
            link_with_prefixes::<OvertimeApprovalsRouteTag>(
                "overtime-queue",
                "Approve overtime",
                65,
                None,
                OVERTIME_QUEUE_MATCH_PREFIXES,
            )
            .with_visible(manager_queue_visible)
            .with_roles(roles_for::<OvertimeView>),
        )
    }
}
