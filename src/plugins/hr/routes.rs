use super::{
    handlers,
    keys::{
        ApplicantDeleteModalKey, ApplicantHubTableKey, EmployeeDeleteModalKey,
        JobFormDeleteModalKey, JobFormSelectModalKey, JobFormSelectTableKey, JobFormTableKey,
    },
};

crate::define_plugin_routes! {
    plugin: HrTag;
    prefix: "/dashboard";
    routes: [
        get ApplicantHubRouteTag, "/hr/applicants", handlers::applicants::hub, fragment(ApplicantHubTableKey);
        get ApplicantCreateGetRouteTag, "/hr/applicants/create", handlers::applicants::create_get, modal;
        post ApplicantCreatePostRouteTag, "/hr/applicants/create", handlers::applicants::create_post;
        get ApplicantDetailRouteTag, "/hr/applicants/{id}", handlers::applicants::detail;
        get ApplicantEditGetRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_get, modal;
        post ApplicantEditPostRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_post;
        get ApplicantDeleteGetRouteTag, "/hr/applicants/{id}/delete", handlers::applicants::delete_get, modal;
        post ApplicantDeletePostRouteTag, "/hr/applicants/{id}/delete", bare handlers::applicants::delete_post, fragment(ApplicantDeleteModalKey);

        get HireApplicantGetRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_get, modal;
        post HireApplicantPostRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_post;

        get EmployeeCreateGetRouteTag, "/hr/employees/create", handlers::employees::create_get, modal;
        post EmployeeCreatePostRouteTag, "/hr/employees/create", handlers::employees::create_post;
        get EmployeeDetailRouteTag, "/hr/employees/{id}", handlers::employees::detail;
        get EmployeeEditGetRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_get, modal;
        post EmployeeEditPostRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_post;
        get EmployeeDeleteGetRouteTag, "/hr/employees/{id}/delete", handlers::employees::delete_get, modal;
        post EmployeeDeletePostRouteTag, "/hr/employees/{id}/delete", bare handlers::employees::delete_post, fragment(EmployeeDeleteModalKey);
        get TerminateEmployeeGetRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_get, modal;
        post TerminateEmployeePostRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_post;

        get ExEmployeeCreateGetRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_get, modal;
        post ExEmployeeCreatePostRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_post;
        get ExEmployeeDetailRouteTag, "/hr/ex-employees/{id}", handlers::ex_employees::detail;

        get JobFormListRouteTag, "/hr/job-forms", handlers::job_forms::list, fragment(JobFormTableKey);
        get JobFormCreateGetRouteTag, "/hr/job-forms/create", handlers::job_forms::create_get, modal;
        post JobFormCreatePostRouteTag, "/hr/job-forms/create", handlers::job_forms::create_post;
        get JobFormDetailRouteTag, "/hr/job-forms/{id}", handlers::job_forms::detail;
        get JobFormEditGetRouteTag, "/hr/job-forms/{id}/edit", handlers::job_forms::edit_get, modal;
        post JobFormEditPostRouteTag, "/hr/job-forms/{id}/edit", handlers::job_forms::edit_post;
        get JobFormDeleteGetRouteTag, "/hr/job-forms/{id}/delete", handlers::job_forms::delete_get, modal;
        post JobFormDeletePostRouteTag, "/hr/job-forms/{id}/delete", bare handlers::job_forms::delete_post, fragment(JobFormDeleteModalKey);
        get JobFormFkSelectRouteTag, "/hr/job-forms/pick", handlers::job_forms::select, fk_select(JobFormSelectTableKey, JobFormSelectModalKey);

        get JobApplicationPublicGetRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_get, raw;
        post JobApplicationPublicPostRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_post, raw;

        get HrDashboardGetRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_get, raw;
        post HrDashboardPostRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_post, raw;
    ]
}
