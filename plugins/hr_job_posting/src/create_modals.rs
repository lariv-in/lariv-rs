//! Typed [`CreateModal`] wiring for job posting swap keys.

use super::keys::{ApplicantCreateModalKey, JobFormCreateModalKey};
use super::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, JobFormCreateGetRouteTag,
    JobFormCreatePostRouteTag,
};

lariv_core::impl_create_modal!(
    ApplicantCreateModalKey,
    ApplicantCreateGetRouteTag,
    ApplicantCreatePostRouteTag,
    "p_hr.ApplicantCreateForm"
);
lariv_core::impl_create_modal!(
    JobFormCreateModalKey,
    JobFormCreateGetRouteTag,
    JobFormCreatePostRouteTag,
    "p_hr.JobFormCreateForm"
);
