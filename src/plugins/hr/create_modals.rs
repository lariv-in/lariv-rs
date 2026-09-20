//! Typed [`CreateModal`] wiring for HR swap keys.

use super::keys::{
    ApplicantCreateModalKey, EmployeeCreateModalKey, ExEmployeeCreateModalKey,
    JobFormCreateModalKey, ProbationCreateModalKey,
};
use super::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, EmployeeCreateGetRouteTag,
    EmployeeCreatePostRouteTag, ExEmployeeCreateGetRouteTag, ExEmployeeCreatePostRouteTag,
    JobFormCreateGetRouteTag, JobFormCreatePostRouteTag, ProbationCreateGetRouteTag,
    ProbationCreatePostRouteTag,
};

crate::impl_create_modal!(
    ApplicantCreateModalKey,
    ApplicantCreateGetRouteTag,
    ApplicantCreatePostRouteTag,
    "p_hr.ApplicantCreateForm"
);
crate::impl_create_modal!(
    ProbationCreateModalKey,
    ProbationCreateGetRouteTag,
    ProbationCreatePostRouteTag,
    "p_hr.ProbationCreateForm"
);
crate::impl_create_modal!(
    EmployeeCreateModalKey,
    EmployeeCreateGetRouteTag,
    EmployeeCreatePostRouteTag,
    "p_hr.EmployeeCreateForm"
);
crate::impl_create_modal!(
    ExEmployeeCreateModalKey,
    ExEmployeeCreateGetRouteTag,
    ExEmployeeCreatePostRouteTag,
    "p_hr.ExEmployeeCreateForm"
);
crate::impl_create_modal!(
    JobFormCreateModalKey,
    JobFormCreateGetRouteTag,
    JobFormCreatePostRouteTag,
    "p_hr.JobFormCreateForm"
);
