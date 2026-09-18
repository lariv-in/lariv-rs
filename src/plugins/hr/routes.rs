use super::{
    handlers,
    keys::{ApplicantDeleteModalKey, ApplicantHubTableKey},
};

crate::define_plugin_routes! {
    plugin: HrTag;
    routes: [
        get ApplicantHubRouteTag, "/hr/applicants", handlers::applicants::hub, fragment(ApplicantHubTableKey);
        get ApplicantCreateGetRouteTag, "/hr/applicants/create", handlers::applicants::create_get, modal;
        post ApplicantCreatePostRouteTag, "/hr/applicants/create", handlers::applicants::create_post;
        get ApplicantDetailRouteTag, "/hr/applicants/{id}", handlers::applicants::detail;
        get ApplicantEditGetRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_get, modal;
        post ApplicantEditPostRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_post;
        get ApplicantDeleteGetRouteTag, "/hr/applicants/{id}/delete", handlers::applicants::delete_get, modal;
        post ApplicantDeletePostRouteTag, "/hr/applicants/{id}/delete", bare handlers::applicants::delete_post, fragment(ApplicantDeleteModalKey);

        get StartProbationGetRouteTag, "/hr/applicants/{id}/start-probation", handlers::applicants::start_probation_get, modal;
        post StartProbationPostRouteTag, "/hr/applicants/{id}/start-probation", handlers::applicants::start_probation_post;

        get ProbationCreateGetRouteTag, "/hr/probations/create", handlers::probations::create_get, modal;
        post ProbationCreatePostRouteTag, "/hr/probations/create", handlers::probations::create_post;
        get ProbationDetailRouteTag, "/hr/probations/{id}", handlers::probations::detail;
        get ProbationEditGetRouteTag, "/hr/probations/{id}/edit", handlers::probations::edit_get, modal;
        post ProbationEditPostRouteTag, "/hr/probations/{id}/edit", handlers::probations::edit_post;
        get HireEmployeeGetRouteTag, "/hr/probations/{id}/hire", handlers::probations::hire_get, modal;
        post HireEmployeePostRouteTag, "/hr/probations/{id}/hire", handlers::probations::hire_post;

        get EmployeeCreateGetRouteTag, "/hr/employees/create", handlers::employees::create_get, modal;
        post EmployeeCreatePostRouteTag, "/hr/employees/create", handlers::employees::create_post;
        get EmployeeDetailRouteTag, "/hr/employees/{id}", handlers::employees::detail;
        get EmployeeEditGetRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_get, modal;
        post EmployeeEditPostRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_post;
        get TerminateEmployeeGetRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_get, modal;
        post TerminateEmployeePostRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_post;

        get ExEmployeeCreateGetRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_get, modal;
        post ExEmployeeCreatePostRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_post;
        get ExEmployeeDetailRouteTag, "/hr/ex-employees/{id}", handlers::ex_employees::detail;
    ]
}
