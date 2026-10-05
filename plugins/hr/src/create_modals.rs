//! Typed [`CreateModal`] wiring for HR swap keys.

use super::keys::{
    ApplicantCreateModalKey, EmployeeCreateModalKey, ExEmployeeCreateModalKey,
    HolidayCreateModalKey, JobFormCreateModalKey,
};
use super::routes::{
    ApplicantCreateGetRouteTag, ApplicantCreatePostRouteTag, EmployeeCreateGetRouteTag,
    EmployeeCreatePostRouteTag, ExEmployeeCreateGetRouteTag, ExEmployeeCreatePostRouteTag,
    HolidayCreateGetRouteTag, HolidayCreatePostRouteTag, JobFormCreateGetRouteTag,
    JobFormCreatePostRouteTag,
};

lariv_core::impl_create_modal!(
    ApplicantCreateModalKey,
    ApplicantCreateGetRouteTag,
    ApplicantCreatePostRouteTag,
    "p_hr.ApplicantCreateForm"
);
lariv_core::impl_create_modal!(
    EmployeeCreateModalKey,
    EmployeeCreateGetRouteTag,
    EmployeeCreatePostRouteTag,
    "p_hr.EmployeeCreateForm"
);
lariv_core::impl_create_modal!(
    ExEmployeeCreateModalKey,
    ExEmployeeCreateGetRouteTag,
    ExEmployeeCreatePostRouteTag,
    "p_hr.ExEmployeeCreateForm"
);
lariv_core::impl_create_modal!(
    JobFormCreateModalKey,
    JobFormCreateGetRouteTag,
    JobFormCreatePostRouteTag,
    "p_hr.JobFormCreateForm"
);
lariv_core::impl_create_modal!(
    HolidayCreateModalKey,
    HolidayCreateGetRouteTag,
    HolidayCreatePostRouteTag,
    "p_hr.HolidayCreateForm"
);
