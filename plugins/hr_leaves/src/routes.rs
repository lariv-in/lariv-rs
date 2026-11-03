use lariv_plugin_hr::routes::{EmployeeMutate, HrPeopleView};
use lariv_plugin_users::roles::Unassigned;

use super::{
    handlers,
    keys::{LEAVE_DELETE_MODAL, LEAVE_JOURNAL_TABLE, LEAVE_TABLE},
};

pub struct LeaveView;

pub struct LeaveMutate;

lariv_core::define_plugin_routes! {
    plugin: HrLeavesTag;
    prefix: "/dashboard";
    routes: [
        get LeaveListRouteTag, "/hr/leaves", handlers::leaves::list, fragment(LEAVE_TABLE), authorize(LeaveView, [Unassigned]);
        get LeaveApplicationsRouteTag, "/hr/leaves/applications", handlers::leaves::applications, fragment(LEAVE_TABLE), authorize(LeaveView, [Unassigned]);
        get LeaveApprovedRouteTag, "/hr/leaves/approved", handlers::leaves::approved, fragment(LEAVE_TABLE), authorize(LeaveView, [Unassigned]);
        get LeaveRejectedRouteTag, "/hr/leaves/rejected", handlers::leaves::rejected, fragment(LEAVE_TABLE), authorize(LeaveView, [Unassigned]);
        get LeaveApprovalsRouteTag, "/hr/leaves/approvals", handlers::leaves::approvals, fragment(LEAVE_TABLE), authorize(LeaveView, [Unassigned]);
        get LeaveCreateGetRouteTag, "/hr/leaves/create", handlers::leaves::create_get, modal, authorize(LeaveView, [Unassigned]);
        post LeaveCreatePostRouteTag, "/hr/leaves/create", handlers::leaves::create_post, authorize(LeaveView, [Unassigned]);
        get LeaveDetailRouteTag, "/hr/leaves/{id}", handlers::leaves::detail, authorize(LeaveView, [Unassigned]);
        get LeaveEditGetRouteTag, "/hr/leaves/{id}/edit", handlers::leaves::edit_get, modal, authorize(LeaveMutate, []);
        post LeaveEditPostRouteTag, "/hr/leaves/{id}/edit", handlers::leaves::edit_post, authorize(LeaveMutate, []);
        get LeaveDeleteGetRouteTag, "/hr/leaves/{id}/delete", handlers::leaves::delete_get, modal, authorize(LeaveMutate, []);
        post LeaveDeletePostRouteTag, "/hr/leaves/{id}/delete", bare handlers::leaves::delete_post, fragment(LEAVE_DELETE_MODAL), authorize(LeaveMutate, []);
        get LeaveApproveGetRouteTag, "/hr/leaves/{id}/approve", handlers::leaves::approve_get, modal, authorize(LeaveView, [Unassigned]);
        post LeaveApprovePostRouteTag, "/hr/leaves/{id}/approve", handlers::leaves::approve_post, authorize(LeaveView, [Unassigned]);
        get LeaveRejectGetRouteTag, "/hr/leaves/{id}/reject", handlers::leaves::reject_get, modal, authorize(LeaveMutate, []);
        post LeaveRejectPostRouteTag, "/hr/leaves/{id}/reject", handlers::leaves::reject_post, authorize(LeaveMutate, []);
        get LeaveRevokeApprovalGetRouteTag, "/hr/leaves/{id}/revoke-approval", handlers::leaves::revoke_approval_get, modal, authorize(LeaveView, [Unassigned]);
        post LeaveRevokeApprovalPostRouteTag, "/hr/leaves/{id}/revoke-approval", handlers::leaves::revoke_approval_post, authorize(LeaveView, [Unassigned]);
        get LeaveRevokeRejectionGetRouteTag, "/hr/leaves/{id}/revoke-rejection", handlers::leaves::revoke_rejection_get, modal, authorize(LeaveMutate, []);
        post LeaveRevokeRejectionPostRouteTag, "/hr/leaves/{id}/revoke-rejection", handlers::leaves::revoke_rejection_post, authorize(LeaveMutate, []);

        get EmployeeLeaveJournalRouteTag, "/hr/employees/{id}/leave-journal", handlers::employees::leave_journal, fragment(LEAVE_JOURNAL_TABLE), authorize(HrPeopleView, []);
        get EmployeeGiveLeaveGetRouteTag, "/hr/employees/{id}/leave-journal/give", handlers::employees::give_leave_get, modal, authorize(EmployeeMutate, []);
        post EmployeeGiveLeavePostRouteTag, "/hr/employees/{id}/leave-journal/give", handlers::employees::give_leave_post, authorize(EmployeeMutate, []);
    ]
}
