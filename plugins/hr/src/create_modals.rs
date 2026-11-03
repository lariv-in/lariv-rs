//! Typed [`CreateModal`] wiring for HR swap keys.

use lariv_core::web::CreateModal;

use super::keys::{EMPLOYEE_CREATE_MODAL, EX_EMPLOYEE_CREATE_MODAL};
use super::routes::{
    EmployeeCreateGetRouteTag, EmployeeCreatePostRouteTag, ExEmployeeCreateGetRouteTag,
    ExEmployeeCreatePostRouteTag,
};

pub const EMPLOYEE_CREATE: CreateModal<EmployeeCreateGetRouteTag, EmployeeCreatePostRouteTag> =
    CreateModal {
        key: EMPLOYEE_CREATE_MODAL,
        form_name: "p_hr.EmployeeCreateForm",
        get: EmployeeCreateGetRouteTag,
        post: EmployeeCreatePostRouteTag,
    };

pub const EX_EMPLOYEE_CREATE: CreateModal<
    ExEmployeeCreateGetRouteTag,
    ExEmployeeCreatePostRouteTag,
> = CreateModal {
    key: EX_EMPLOYEE_CREATE_MODAL,
    form_name: "p_hr.ExEmployeeCreateForm",
    get: ExEmployeeCreateGetRouteTag,
    post: ExEmployeeCreatePostRouteTag,
};
