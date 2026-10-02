use super::{
    handlers,
    keys::{
        ApplicantDeleteModalKey, ApplicantHubTableKey, AttendanceDeleteModalKey,
        AttendanceTableKey, EmployeeDeleteModalKey, HolidayDeleteModalKey, HolidayTableKey,
        JobFormDeleteModalKey, JobFormSelectModalKey, JobFormSelectTableKey, JobFormTableKey,
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

crate::define_plugin_routes! {
    plugin: HrTag;
    prefix: "/dashboard";
    routes: [
        get ApplicantHubRouteTag, "/hr/applicants", handlers::applicants::hub, fragment(ApplicantHubTableKey), authorize(HrPeopleView, ["applicant", "probation", "employee", "ex-employee"]);
        get ApplicantCreateGetRouteTag, "/hr/applicants/create", handlers::applicants::create_get, modal, authorize(ApplicantMutate, []);
        post ApplicantCreatePostRouteTag, "/hr/applicants/create", handlers::applicants::create_post, authorize(ApplicantMutate, []);
        get ApplicantDetailRouteTag, "/hr/applicants/{id}", handlers::applicants::detail, authorize(HrPeopleView, ["applicant", "probation", "employee", "ex-employee"]);
        get ApplicantEditGetRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_get, modal, authorize(ApplicantMutate, []);
        post ApplicantEditPostRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_post, authorize(ApplicantMutate, []);
        get ApplicantDeleteGetRouteTag, "/hr/applicants/{id}/delete", handlers::applicants::delete_get, modal, authorize(ApplicantMutate, []);
        post ApplicantDeletePostRouteTag, "/hr/applicants/{id}/delete", bare handlers::applicants::delete_post, fragment(ApplicantDeleteModalKey), authorize(ApplicantMutate, []);

        get HireApplicantGetRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_get, modal, authorize(ApplicantMutate, []);
        post HireApplicantPostRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_post, authorize(ApplicantMutate, []);

        get EmployeeCreateGetRouteTag, "/hr/employees/create", handlers::employees::create_get, modal, authorize(EmployeeMutate, []);
        post EmployeeCreatePostRouteTag, "/hr/employees/create", handlers::employees::create_post, authorize(EmployeeMutate, []);
        get EmployeeDetailRouteTag, "/hr/employees/{id}", handlers::employees::detail, authorize(HrPeopleView, ["applicant", "probation", "employee", "ex-employee"]);
        get EmployeeEditGetRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_get, modal, authorize(EmployeeMutate, []);
        post EmployeeEditPostRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_post, authorize(EmployeeMutate, []);
        get EmployeeDeleteGetRouteTag, "/hr/employees/{id}/delete", handlers::employees::delete_get, modal, authorize(EmployeeMutate, []);
        post EmployeeDeletePostRouteTag, "/hr/employees/{id}/delete", bare handlers::employees::delete_post, fragment(EmployeeDeleteModalKey), authorize(EmployeeMutate, []);
        get TerminateEmployeeGetRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_get, modal, authorize(EmployeeMutate, []);
        post TerminateEmployeePostRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_post, authorize(EmployeeMutate, []);

        get ExEmployeeCreateGetRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_get, modal, authorize(ExEmployeeMutate, []);
        post ExEmployeeCreatePostRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_post, authorize(ExEmployeeMutate, []);
        get ExEmployeeDetailRouteTag, "/hr/ex-employees/{id}", handlers::ex_employees::detail, authorize(HrPeopleView, ["applicant", "probation", "employee", "ex-employee"]);

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

        get AttendanceListRouteTag, "/hr/attendances", handlers::attendances::list, fragment(AttendanceTableKey), authorize(AttendanceView, []);
        get AttendanceCreateGetRouteTag, "/hr/attendances/create", handlers::attendances::create_get, modal, authorize(AttendanceMutate, []);
        post AttendanceCreatePostRouteTag, "/hr/attendances/create", handlers::attendances::create_post, authorize(AttendanceMutate, []);
        get AttendanceDetailRouteTag, "/hr/attendances/{id}", handlers::attendances::detail, authorize(AttendanceView, []);
        get AttendanceEditGetRouteTag, "/hr/attendances/{id}/edit", handlers::attendances::edit_get, modal, authorize(AttendanceMutate, []);
        post AttendanceEditPostRouteTag, "/hr/attendances/{id}/edit", handlers::attendances::edit_post, authorize(AttendanceMutate, []);
        get AttendanceDeleteGetRouteTag, "/hr/attendances/{id}/delete", handlers::attendances::delete_get, modal, authorize(AttendanceMutate, []);
        post AttendanceDeletePostRouteTag, "/hr/attendances/{id}/delete", bare handlers::attendances::delete_post, fragment(AttendanceDeleteModalKey), authorize(AttendanceMutate, []);

        get JobApplicationPublicGetRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_get, raw;
        post JobApplicationPublicPostRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_post, raw;

        get HrDashboardGetRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_get, raw;
        post HrDashboardPostRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_post, raw;
    ]
}
