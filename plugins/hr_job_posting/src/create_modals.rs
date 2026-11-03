//! Typed [`CreateModal`] wiring for job posting swap keys.

use lariv_core::web::CreateModal;

use super::keys::{APPLICANT_CREATE_MODAL, JOB_FORM_CREATE_MODAL};
use super::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, JobFormCreateGetRouteTag,
    JobFormCreatePostRouteTag,
};

pub const APPLICANT_CREATE: CreateModal<ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag> =
    CreateModal {
        key: APPLICANT_CREATE_MODAL,
        form_name: "p_hr.ApplicantCreateForm",
        get: ApplicantCreateGetRouteTag,
        post: ApplicantCreatePostRouteTag,
    };

pub const JOB_FORM_CREATE: CreateModal<JobFormCreateGetRouteTag, JobFormCreatePostRouteTag> =
    CreateModal {
        key: JOB_FORM_CREATE_MODAL,
        form_name: "p_hr.JobFormCreateForm",
        get: JobFormCreateGetRouteTag,
        post: JobFormCreatePostRouteTag,
    };
