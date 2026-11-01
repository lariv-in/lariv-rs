use super::{
    handlers,
    keys::{ApplicantHubTableKey, EmployeeDeleteModalKey},
};

pub struct HrPeopleView;

pub struct EmployeeMutate;

pub struct ExEmployeeMutate;

/// HR preferences. Empty allowlist: superuser only.
pub struct HrPrefsAdmin;

lariv_core::define_plugin_routes! {
    plugin: HrTag;
    prefix: "/dashboard";
    routes: [
        get PeopleHubRouteTag, "/hr/people", handlers::applicants::hub, fragment(ApplicantHubTableKey), authorize(HrPeopleView, []);

        get EmployeeCreateGetRouteTag, "/hr/employees/create", handlers::employees::create_get, modal, authorize(EmployeeMutate, []);
        post EmployeeCreatePostRouteTag, "/hr/employees/create", handlers::employees::create_post, authorize(EmployeeMutate, []);
        get EmployeeDetailRouteTag, "/hr/employees/{id}", handlers::employees::detail, authorize(HrPeopleView, []);
        get EmployeeEditGetRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_get, modal, authorize(EmployeeMutate, []);
        post EmployeeEditPostRouteTag, "/hr/employees/{id}/edit", handlers::employees::edit_post, authorize(EmployeeMutate, []);
        get EmployeeDeleteGetRouteTag, "/hr/employees/{id}/delete", handlers::employees::delete_get, modal, authorize(EmployeeMutate, []);
        post EmployeeDeletePostRouteTag, "/hr/employees/{id}/delete", bare handlers::employees::delete_post, fragment(EmployeeDeleteModalKey), authorize(EmployeeMutate, []);
        get TerminateEmployeeGetRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_get, modal, authorize(EmployeeMutate, []);
        post TerminateEmployeePostRouteTag, "/hr/employees/{id}/terminate", handlers::employees::terminate_post, authorize(EmployeeMutate, []);
        get EmployeeIdCardModalRouteTag, "/hr/employees/{id}/id-card", bare handlers::id_card::modal, modal, authorize(HrPeopleView, []);
        get EmployeeIdCardPdfRouteTag, "/hr/employees/{id}/id-card/file", bare handlers::id_card::pdf, file, authorize(HrPeopleView, []);

        get ExEmployeeCreateGetRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_get, modal, authorize(ExEmployeeMutate, []);
        post ExEmployeeCreatePostRouteTag, "/hr/ex-employees/create", handlers::ex_employees::create_post, authorize(ExEmployeeMutate, []);
        get ExEmployeeDetailRouteTag, "/hr/ex-employees/{id}", handlers::ex_employees::detail, authorize(HrPeopleView, []);



        get HrHomeRouteTag, "/hr", bare handlers::home::home, redirect, authorize(HrPeopleView, []);
        get HrLeavePrefsGetRouteTag, "/hr/preferences", handlers::preferences::get, authorize(HrPrefsAdmin, []);
        post HrLeavePrefsPostRouteTag, "/hr/preferences", handlers::preferences::post, authorize(HrPrefsAdmin, []);
        post HrIdCardPreviewPostRouteTag, "/hr/preferences/id-card-preview", bare handlers::id_card::preview_post, modal, authorize(HrPrefsAdmin, []);
        get HrIdCardPreviewPdfRouteTag, "/hr/preferences/id-card-preview/{token}", bare handlers::id_card::preview_pdf, file, param token: String, authorize(HrPrefsAdmin, []);
        get HrIdCardJwksRouteTag, "/hr/id-card/jwks", root bare handlers::id_card::jwks, raw;

        get HrDashboardGetRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_get, raw;
        post HrDashboardPostRouteTag, "/dashboard", root bare handlers::dashboard::dashboard_post, raw;
    ]
}
