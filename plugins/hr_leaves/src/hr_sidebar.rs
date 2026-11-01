//! Leaves links patched onto the shared HR app sidebar.

use lariv_plugin_hr::hr_employee_detail::{HrEmployeeDetailLink, HrEmployeeDetailRegistry};
use lariv_plugin_hr::hr_sidebar::{HrSidebarRegistrar, HrSidebarRegistry, link_with_prefixes};
use lariv_plugin_users::role_authorization::{current_auth, roles_for};
use lariv_plugin_users::roles::Superuser;

use crate::routes::{
    EmployeeLeaveJournalRouteTag, LeaveApplicationsRouteTag, LeaveApprovalsRouteTag,
    LeaveListRouteTag, LeaveView,
};

const LEAVES_MATCH_PREFIXES: &[&str] = &[LeaveListRouteTag::PATH];
const LEAVE_QUEUE_MATCH_PREFIXES: &[&str] = &[LeaveApprovalsRouteTag::PATH];

fn self_service() -> bool {
    current_auth().is_some_and(|auth| !Superuser::matches(&auth.role))
}

fn leaves_url() -> String {
    if self_service() {
        LeaveApplicationsRouteTag.url()
    } else {
        LeaveListRouteTag.url()
    }
}

fn manager_queue_visible() -> bool {
    self_service() && lariv_plugin_hr::nav::leave_approvals_visible()
}

fn leave_journal_url(employee_id: i64) -> String {
    EmployeeLeaveJournalRouteTag::new(employee_id).url()
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl HrSidebarRegistrar for Hook {
    fn register_hr_sidebar(self, cap: HrSidebarRegistry) -> HrSidebarRegistry {
        cap.push(
            link_with_prefixes::<LeaveListRouteTag>(
                "leaves",
                "Leaves",
                50,
                None,
                LEAVES_MATCH_PREFIXES,
            )
            .with_url_resolver(leaves_url)
            .with_roles(roles_for::<LeaveView>),
        )
        .push(
            link_with_prefixes::<LeaveApprovalsRouteTag>(
                "leave-queue",
                "Approve leaves",
                55,
                None,
                LEAVE_QUEUE_MATCH_PREFIXES,
            )
            .with_visible(manager_queue_visible)
            .with_roles(roles_for::<LeaveView>),
        )
    }

    fn register_employee_detail_links(
        self,
        cap: HrEmployeeDetailRegistry,
    ) -> HrEmployeeDetailRegistry {
        cap.push(HrEmployeeDetailLink {
            key: "leave-journal",
            title: "Leaves journal",
            url_fn: leave_journal_url,
            order: 10,
        })
    }
}
