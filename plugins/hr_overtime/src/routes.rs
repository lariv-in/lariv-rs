use lariv_plugin_users::roles::Unassigned;

use super::{
    handlers,
    keys::{APPROVED_OVERTIME_TABLE, OVERTIME_DELETE_MODAL, OVERTIME_TABLE},
};

pub struct OvertimeView;

/// Direct entry of an approved overtime row. Empty allowlist: superuser only.
pub struct ApprovedOvertimeMutate;

lariv_core::define_plugin_routes! {
    plugin: HrOvertimeTag;
    prefix: "/dashboard";
    routes: [
        get OvertimeListRouteTag, "/hr/overtime", handlers::overtime::list, fragment(OVERTIME_TABLE), authorize(OvertimeView, [Unassigned]);
        get OvertimeApplicationsRouteTag, "/hr/overtime/applications", handlers::overtime::applications, fragment(OVERTIME_TABLE), authorize(OvertimeView, [Unassigned]);
        get OvertimeApprovalsRouteTag, "/hr/overtime/approvals", handlers::overtime::approvals, fragment(OVERTIME_TABLE), authorize(OvertimeView, [Unassigned]);
        get OvertimeApprovedRouteTag, "/hr/overtime/approved", handlers::overtime::approved, fragment(APPROVED_OVERTIME_TABLE), authorize(OvertimeView, [Unassigned]);
        get OvertimeApprovedCreateGetRouteTag, "/hr/overtime/approved/create", handlers::overtime::approved_create_get, modal, authorize(ApprovedOvertimeMutate, []);
        post OvertimeApprovedCreatePostRouteTag, "/hr/overtime/approved/create", handlers::overtime::approved_create_post, authorize(ApprovedOvertimeMutate, []);
        get OvertimeCreateGetRouteTag, "/hr/overtime/create", handlers::overtime::create_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeCreatePostRouteTag, "/hr/overtime/create", handlers::overtime::create_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeDetailRouteTag, "/hr/overtime/{id}", handlers::overtime::detail, authorize(OvertimeView, [Unassigned]);
        get OvertimeEditGetRouteTag, "/hr/overtime/{id}/edit", handlers::overtime::edit_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeEditPostRouteTag, "/hr/overtime/{id}/edit", handlers::overtime::edit_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeDeleteGetRouteTag, "/hr/overtime/{id}/delete", handlers::overtime::delete_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeDeletePostRouteTag, "/hr/overtime/{id}/delete", bare handlers::overtime::delete_post, fragment(OVERTIME_DELETE_MODAL), authorize(OvertimeView, [Unassigned]);
        get OvertimeApproveGetRouteTag, "/hr/overtime/{id}/approve", handlers::overtime::approve_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeApprovePostRouteTag, "/hr/overtime/{id}/approve", handlers::overtime::approve_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeRejectGetRouteTag, "/hr/overtime/{id}/reject", handlers::overtime::reject_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeRejectPostRouteTag, "/hr/overtime/{id}/reject", handlers::overtime::reject_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeRevokeApprovalGetRouteTag, "/hr/overtime/{id}/revoke-approval", handlers::overtime::revoke_approval_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeRevokeApprovalPostRouteTag, "/hr/overtime/{id}/revoke-approval", handlers::overtime::revoke_approval_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeRevokeRejectionGetRouteTag, "/hr/overtime/{id}/revoke-rejection", handlers::overtime::revoke_rejection_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeRevokeRejectionPostRouteTag, "/hr/overtime/{id}/revoke-rejection", handlers::overtime::revoke_rejection_post, authorize(OvertimeView, [Unassigned]);
    ]
}
