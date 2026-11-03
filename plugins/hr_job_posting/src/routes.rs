use lariv_plugin_hr::routes::HrPeopleView;

use super::{
    handlers,
    keys::{
        APPLICANT_DELETE_MODAL, APPLICANT_TABLE, JOB_FORM_DELETE_MODAL, JOB_FORM_SELECT_MODAL,
        JOB_FORM_SELECT_TABLE, JOB_FORM_TABLE,
    },
};

pub struct ApplicantMutate;

pub struct JobFormView;

pub struct JobFormMutate;

lariv_core::define_plugin_routes! {
    plugin: HrJobPostingTag;
    prefix: "/dashboard";
    routes: [
        get ApplicantListRouteTag, "/hr/applicants", handlers::applicants::list, fragment(APPLICANT_TABLE), authorize(HrPeopleView, []);
        get ApplicantCreateGetRouteTag, "/hr/applicants/create", handlers::applicants::create_get, modal, authorize(ApplicantMutate, []);
        post ApplicantCreatePostRouteTag, "/hr/applicants/create", handlers::applicants::create_post, authorize(ApplicantMutate, []);
        get ApplicantDetailRouteTag, "/hr/applicants/{id}", handlers::applicants::detail, authorize(HrPeopleView, []);
        get ApplicantEditGetRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_get, modal, authorize(ApplicantMutate, []);
        post ApplicantEditPostRouteTag, "/hr/applicants/{id}/edit", handlers::applicants::edit_post, authorize(ApplicantMutate, []);
        get ApplicantDeleteGetRouteTag, "/hr/applicants/{id}/delete", handlers::applicants::delete_get, modal, authorize(ApplicantMutate, []);
        post ApplicantDeletePostRouteTag, "/hr/applicants/{id}/delete", bare handlers::applicants::delete_post, fragment(APPLICANT_DELETE_MODAL), authorize(ApplicantMutate, []);

        get HireApplicantGetRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_get, modal, authorize(ApplicantMutate, []);
        post HireApplicantPostRouteTag, "/hr/applicants/{id}/hire", handlers::applicants::hire_post, authorize(ApplicantMutate, []);

        get JobFormListRouteTag, "/hr/job-forms", handlers::job_forms::list, fragment(JOB_FORM_TABLE), authorize(JobFormView, []);
        get JobFormCreateGetRouteTag, "/hr/job-forms/create", handlers::job_forms::create_get, modal, authorize(JobFormMutate, []);
        post JobFormCreatePostRouteTag, "/hr/job-forms/create", handlers::job_forms::create_post, authorize(JobFormMutate, []);
        get JobFormDetailRouteTag, "/hr/job-forms/{id}", handlers::job_forms::detail, authorize(JobFormView, []);
        get JobFormEditGetRouteTag, "/hr/job-forms/{id}/edit", handlers::job_forms::edit_get, modal, authorize(JobFormMutate, []);
        post JobFormEditPostRouteTag, "/hr/job-forms/{id}/edit", handlers::job_forms::edit_post, authorize(JobFormMutate, []);
        get JobFormDeleteGetRouteTag, "/hr/job-forms/{id}/delete", handlers::job_forms::delete_get, modal, authorize(JobFormMutate, []);
        post JobFormDeletePostRouteTag, "/hr/job-forms/{id}/delete", bare handlers::job_forms::delete_post, fragment(JOB_FORM_DELETE_MODAL), authorize(JobFormMutate, []);
        get JobFormFkSelectRouteTag, "/hr/job-forms/pick", handlers::job_forms::select, fk_select(JOB_FORM_SELECT_TABLE, JOB_FORM_SELECT_MODAL), authorize(JobFormView, []);

        get JobApplicationPublicGetRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_get, raw;
        post JobApplicationPublicPostRouteTag, "/jobs/{id}/apply", root bare handlers::applications::apply_post, raw;
    ]
}
