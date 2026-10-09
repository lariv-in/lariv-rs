use lariv_plugin_users::roles::Unassigned;

use super::{
    handlers,
    keys::{
        ApplicantDeleteModalKey, ApplicantHubTableKey, ApprovedOvertimeTableKey,
        AttendanceDeleteModalKey, AttendanceTableKey, EmployeeDeleteModalKey,
        HolidayDeleteModalKey, HolidayTableKey, JobFormDeleteModalKey, JobFormSelectModalKey,
        JobFormSelectTableKey, JobFormTableKey, LeaveDeleteModalKey, LeaveJournalTableKey,
        LeaveTableKey, OvertimeDeleteModalKey, OvertimeTableKey,
    },
};

pub struct HrPeopleView;

pub struct ApplicantMutate;

pub struct EmployeeMutate;

pub struct ExEmployeeMutate;

pub struct JobFormView;

pub struct JobFormMutate;

pub struct HolidayView;

pub struct HolidayMutate;

pub struct AttendanceView;

pub struct AttendanceMutate;

pub struct LeaveView;

pub struct LeaveMutate;

/// Leave calculation preferences. Empty allowlist: superuser only.
pub struct HrPrefsAdmin;

pub struct OvertimeView;

lariv_core::define_plugin_routes! {
    plugin: HrTag;
    prefix: "/dashboard";
    routes: [
        get ApplicantHubRouteTag, "/hr/applicants", handlers::applicants::hub, fragment(ApplicantHubTableKey), authorize(HrPeopleView, []);
        get ApplicantCreateGetRouteTag, "/hr/applicants/create", handlers::applicants::create_get, modal, authorize(ApplicantMutate, []);
        post ApplicantCreatePostRouteTag, "/hr/applicants/create", handlers::applicants::create_post, authorize(ApplicantMutate, []);
        get ApplicantDetailRouteTag, "/hr/applicants/{id}", handlers::applicants::detail, authorize(HrPeopleView, []);
        get ApplicantEditGetRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_get, modal, authorize(ApplicantMutate, []);
        post ApplicantEditPostRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_post, authorize(ApplicantMutate, []);
        get ApplicantDeleteGetRouteTag, "/hr/applicants/{id}/delete", handlers::applicants::delete_get, modal, authorize(ApplicantMutate, []);
        post ApplicantDeletePostRouteTag, "/hr/applicants/{id}/delete", bare handlers::applicants::delete_post, fragment(ApplicantDeleteModalKey), authorize(ApplicantMutate, []);

        get HireApplicantGetRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_get, modal, authorize(ApplicantMutate, []);
        post HireApplicantPostRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_post, authorize(ApplicantMutate, []);

        get EmployeeCreateGetRouteTag, "/hr/employees/create", handlers::employees::create_get, modal, authorize(EmployeeMutate, []);
        post EmployeeCreatePostRouteTag, "/hr/employees/create", handlers::employees::create_post, authorize(EmployeeMutate, []);
        get EmployeeDetailRouteTag, "/hr/employees/{id}", handlers::employees::detail, authorize(HrPeopleView, []);
        get EmployeeLeaveJournalRouteTag, "/hr/employees/{id}/leave-journal", handlers::employees::leave_journal, fragment(LeaveJournalTableKey), authorize(HrPeopleView, []);
        get EmployeeGiveLeaveGetRouteTag, "/hr/employees/{id}/leave-journal/give", handlers::employees::give_leave_get, modal, authorize(EmployeeMutate, []);
        post EmployeeGiveLeavePostRouteTag, "/hr/employees/{id}/leave-journal/give", handlers::employees::give_leave_post, authorize(EmployeeMutate, []);
        get EmployeeEditGetRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_get, modal, authorize(EmployeeMutate, []);
        post EmployeeEditPostRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_post, authorize(EmployeeMutate, []);
        get EmployeeDeleteGetRouteTag, "/hr/employees/{id}/delete", handlers::employees::delete_get, modal, authorize(EmployeeMutate, []);
        post EmployeeDeletePostRouteTag, "/hr/employees/{id}/delete", bare handlers::employees::delete_post, fragment(EmployeeDeleteModalKey), authorize(EmployeeMutate, []);
        get TerminateEmployeeGetRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_get, modal, authorize(EmployeeMutate, []);
        post TerminateEmployeePostRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_post, authorize(EmployeeMutate, []);

        get ExEmployeeCreateGetRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_get, modal, authorize(ExEmployeeMutate, []);
        post ExEmployeeCreatePostRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_post, authorize(ExEmployeeMutate, []);
        get ExEmployeeDetailRouteTag, "/hr/ex-employees/{id}", handlers::ex_employees::detail, authorize(HrPeopleView, []);

        get JobFormListRouteTag, "/hr/job-forms", handlers::job_forms::list, fragment(JobFormTableKey), authorize(JobFormView, []);
        get JobFormCreateGetRouteTag, "/hr/job-forms/create", handlers::job_forms::create_get, modal, authorize(JobFormMutate, []);
        post JobFormCreatePostRouteTag, "/hr/job-forms/create", handlers::job_forms::create_post, authorize(JobFormMutate, []);
        get JobFormDetailRouteTag, "/hr/job-forms/{id}", handlers::job_forms::detail, authorize(JobFormView, []);
        get JobFormEditGetRouteTag, "/hr/job-forms/{id}/edit", handlers::job_forms::edit_get, modal, authorize(JobFormMutate, []);
        post JobFormEditPostRouteTag, "/hr/job-forms/{id}/edit", handlers::job_forms::edit_post, authorize(JobFormMutate, []);
        get JobFormDeleteGetRouteTag, "/hr/job-forms/{id}/delete", handlers::job_forms::delete_get, modal, authorize(JobFormMutate, []);
        post JobFormDeletePostRouteTag, "/hr/job-forms/{id}/delete", bare handlers::job_forms::delete_post, fragment(JobFormDeleteModalKey), authorize(JobFormMutate, []);
        get JobFormFkSelectRouteTag, "/hr/job-forms/pick", handlers::job_forms::select, fk_select(JobFormSelectTableKey, JobFormSelectModalKey), authorize(JobFormView, []);

        get HolidayListRouteTag, "/hr/holidays", handlers::holidays::list, fragment(HolidayTableKey), authorize(HolidayView, []);
        get HolidayCreateGetRouteTag, "/hr/holidays/create", handlers::holidays::create_get, modal, authorize(HolidayMutate, []);
        post HolidayCreatePostRouteTag, "/hr/holidays/create", handlers::holidays::create_post, authorize(HolidayMutate, []);
        get HolidayDetailRouteTag, "/hr/holidays/{id}", handlers::holidays::detail, authorize(HolidayView, []);
        get HolidayEditGetRouteTag, "/hr/holidays/{id}/edit", handlers::holidays::edit_get, modal, authorize(HolidayMutate, []);
        post HolidayEditPostRouteTag, "/hr/holidays/{id}/edit", handlers::holidays::edit_post, authorize(HolidayMutate, []);
        get HolidayDeleteGetRouteTag, "/hr/holidays/{id}/delete", handlers::holidays::delete_get, modal, authorize(HolidayMutate, []);
        post HolidayDeletePostRouteTag, "/hr/holidays/{id}/delete", bare handlers::holidays::delete_post, fragment(HolidayDeleteModalKey), authorize(HolidayMutate, []);

        get AttendanceListRouteTag, "/hr/attendances", handlers::attendances::list, fragment(AttendanceTableKey), authorize(AttendanceView, [Unassigned]);
        get AttendanceCreateGetRouteTag, "/hr/attendances/create", handlers::attendances::create_get, modal, authorize(AttendanceMutate, []);
        post AttendanceCreatePostRouteTag, "/hr/attendances/create", handlers::attendances::create_post, authorize(AttendanceMutate, []);
        post AttendancePunchInRouteTag, "/hr/attendances/punch-in", handlers::attendances::punch_in_post, fragment(AttendanceTableKey), authorize(AttendanceView, [Unassigned]);
        post AttendancePunchOutRouteTag, "/hr/attendances/punch-out", handlers::attendances::punch_out_post, fragment(AttendanceTableKey), authorize(AttendanceView, [Unassigned]);
        get AttendanceDetailRouteTag, "/hr/attendances/{id}", handlers::attendances::detail, authorize(AttendanceView, [Unassigned]);
        get AttendanceEditGetRouteTag, "/hr/attendances/{id}/edit", handlers::attendances::edit_get, modal, authorize(AttendanceMutate, []);
        post AttendanceEditPostRouteTag, "/hr/attendances/{id}/edit", handlers::attendances::edit_post, authorize(AttendanceMutate, []);
        get AttendanceDeleteGetRouteTag, "/hr/attendances/{id}/delete", handlers::attendances::delete_get, modal, authorize(AttendanceMutate, []);
        post AttendanceDeletePostRouteTag, "/hr/attendances/{id}/delete", bare handlers::attendances::delete_post, fragment(AttendanceDeleteModalKey), authorize(AttendanceMutate, []);

        get HrHomeRouteTag, "/hr", bare handlers::home::home, redirect, authorize(HolidayView, []);

        get LeaveListRouteTag, "/hr/leaves", handlers::leaves::list, fragment(LeaveTableKey), authorize(LeaveView, []);
        get LeaveApplicationsRouteTag, "/hr/leaves/applications", handlers::leaves::applications, fragment(LeaveTableKey), authorize(LeaveView, []);
        get LeaveApprovedRouteTag, "/hr/leaves/approved", handlers::leaves::approved, fragment(LeaveTableKey), authorize(LeaveView, []);
        get LeaveRejectedRouteTag, "/hr/leaves/rejected", handlers::leaves::rejected, fragment(LeaveTableKey), authorize(LeaveView, []);
        get LeaveApprovalsRouteTag, "/hr/leaves/approvals", handlers::leaves::approvals, fragment(LeaveTableKey), authorize(LeaveView, []);
        get LeaveCreateGetRouteTag, "/hr/leaves/create", handlers::leaves::create_get, modal, authorize(LeaveView, []);
        post LeaveCreatePostRouteTag, "/hr/leaves/create", handlers::leaves::create_post, authorize(LeaveView, []);
        get LeaveDetailRouteTag, "/hr/leaves/{id}", handlers::leaves::detail, authorize(LeaveView, []);
        get LeaveEditGetRouteTag, "/hr/leaves/{id}/edit", handlers::leaves::edit_get, modal, authorize(LeaveMutate, []);
        post LeaveEditPostRouteTag, "/hr/leaves/{id}/edit", handlers::leaves::edit_post, authorize(LeaveMutate, []);
        get LeaveDeleteGetRouteTag, "/hr/leaves/{id}/delete", handlers::leaves::delete_get, modal, authorize(LeaveMutate, []);
        post LeaveDeletePostRouteTag, "/hr/leaves/{id}/delete", bare handlers::leaves::delete_post, fragment(LeaveDeleteModalKey), authorize(LeaveMutate, []);
        get LeaveApproveGetRouteTag, "/hr/leaves/{id}/approve", handlers::leaves::approve_get, modal, authorize(LeaveView, []);
        post LeaveApprovePostRouteTag, "/hr/leaves/{id}/approve", handlers::leaves::approve_post, authorize(LeaveView, []);
        get LeaveRejectGetRouteTag, "/hr/leaves/{id}/reject", handlers::leaves::reject_get, modal, authorize(LeaveMutate, []);
        post LeaveRejectPostRouteTag, "/hr/leaves/{id}/reject", handlers::leaves::reject_post, authorize(LeaveMutate, []);
        get LeaveRevokeApprovalGetRouteTag, "/hr/leaves/{id}/revoke-approval", handlers::leaves::revoke_approval_get, modal, authorize(LeaveView, []);
        post LeaveRevokeApprovalPostRouteTag, "/hr/leaves/{id}/revoke-approval", handlers::leaves::revoke_approval_post, authorize(LeaveView, []);
        get LeaveRevokeRejectionGetRouteTag, "/hr/leaves/{id}/revoke-rejection", handlers::leaves::revoke_rejection_get, modal, authorize(LeaveMutate, []);
        post LeaveRevokeRejectionPostRouteTag, "/hr/leaves/{id}/revoke-rejection", handlers::leaves::revoke_rejection_post, authorize(LeaveMutate, []);
        get HrLeavePrefsGetRouteTag, "/hr/preferences", handlers::leave_preferences::get, authorize(HrPrefsAdmin, []);
        post HrLeavePrefsPostRouteTag, "/hr/preferences", handlers::leave_preferences::post, authorize(HrPrefsAdmin, []);

        get OvertimeListRouteTag, "/hr/overtime", handlers::overtime::list, fragment(OvertimeTableKey), authorize(OvertimeView, [Unassigned]);
        get OvertimeApplicationsRouteTag, "/hr/overtime/applications", handlers::overtime::applications, fragment(OvertimeTableKey), authorize(OvertimeView, [Unassigned]);
        get OvertimeApprovalsRouteTag, "/hr/overtime/approvals", handlers::overtime::approvals, fragment(OvertimeTableKey), authorize(OvertimeView, [Unassigned]);
        get OvertimeApprovedRouteTag, "/hr/overtime/approved", handlers::overtime::approved, fragment(ApprovedOvertimeTableKey), authorize(OvertimeView, [Unassigned]);
        get OvertimeCreateGetRouteTag, "/hr/overtime/create", handlers::overtime::create_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeCreatePostRouteTag, "/hr/overtime/create", handlers::overtime::create_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeDetailRouteTag, "/hr/overtime/{id}", handlers::overtime::detail, authorize(OvertimeView, [Unassigned]);
        get OvertimeEditGetRouteTag, "/hr/overtime/{id}/edit", handlers::overtime::edit_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeEditPostRouteTag, "/hr/overtime/{id}/edit", handlers::overtime::edit_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeDeleteGetRouteTag, "/hr/overtime/{id}/delete", handlers::overtime::delete_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeDeletePostRouteTag, "/hr/overtime/{id}/delete", bare handlers::overtime::delete_post, fragment(OvertimeDeleteModalKey), authorize(OvertimeView, [Unassigned]);
        get OvertimeApproveGetRouteTag, "/hr/overtime/{id}/approve", handlers::overtime::approve_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeApprovePostRouteTag, "/hr/overtime/{id}/approve", handlers::overtime::approve_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeRejectGetRouteTag, "/hr/overtime/{id}/reject", handlers::overtime::reject_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeRejectPostRouteTag, "/hr/overtime/{id}/reject", handlers::overtime::reject_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeRevokeApprovalGetRouteTag, "/hr/overtime/{id}/revoke-approval", handlers::overtime::revoke_approval_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeRevokeApprovalPostRouteTag, "/hr/overtime/{id}/revoke-approval", handlers::overtime::revoke_approval_post, authorize(OvertimeView, [Unassigned]);
        get OvertimeRevokeRejectionGetRouteTag, "/hr/overtime/{id}/revoke-rejection", handlers::overtime::revoke_rejection_get, modal, authorize(OvertimeView, [Unassigned]);
        post OvertimeRevokeRejectionPostRouteTag, "/hr/overtime/{id}/revoke-rejection", handlers::overtime::revoke_rejection_post, authorize(OvertimeView, [Unassigned]);

        get JobApplicationPublicGetRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_get, raw;
        post JobApplicationPublicPostRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_post, raw;

        get HrDashboardGetRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_get, raw;
        post HrDashboardPostRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_post, raw;
    ]
}
