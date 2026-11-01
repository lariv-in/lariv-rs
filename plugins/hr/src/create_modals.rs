//! Typed [`CreateModal`] wiring for HR swap keys.

use super::keys::{EmployeeCreateModalKey, ExEmployeeCreateModalKey};
use super::routes::{
    EmployeeCreateGetRouteTag, EmployeeCreatePostRouteTag, ExEmployeeCreateGetRouteTag,
    ExEmployeeCreatePostRouteTag,
};

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
