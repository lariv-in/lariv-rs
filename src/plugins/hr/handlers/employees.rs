use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, modal_edit_post_url,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::hr::{
    forms::{PersonForm, TerminateEmployeeBody},
    handlers::{ModalNameQuery, applicants::person_input_from_form},
    keys::{ApplicantEditModalKey, EmployeeCreateModalKey, TerminateEmployeeModalKey},
    logic::{
        employee::{create_employee, update_employee},
        ex_employee::terminate_employee,
    },
    routes::{EmployeeDetailRouteTag, EmployeeEditPostRouteTag, ExEmployeeDetailRouteTag},
    scope::{employee_display_name, find_employee_scoped, format_timestamp},
    state::HrState,
    templates::{
        EmployeeDetailPage, PersonCreateKind, PersonCreateModalPage, PersonEditModalPage,
        TerminateEmployeeModalPage,
    },
};

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let page = PersonCreateModalPage::new(
        q.form_name(),
        q.refresh_table(),
        "New employee",
        "Create employee",
        PersonCreateKind::Employee,
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<PersonForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match create_employee(&state.db, person_input_from_form(&form)).await {
        Ok(employee) => respond_create_modal_done::<EmployeeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &EmployeeDetailRouteTag::new(employee.id).url(),
        ),
        Err(e) => {
            let page = PersonCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                "New employee",
                "Create employee",
                PersonCreateKind::Employee,
                &form,
                e,
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let can_edit = ctx.user.is_superuser;
    let page = EmployeeDetailPage {
        id: employee.id,
        display_name: employee_display_name(&employee),
        name: employee.name,
        mobile: employee.mobile,
        email: employee.email,
        hired_at: format_timestamp(employee.hired_at, &ctx.timezone),
        can_edit,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let form_name = q.form_name();
    let page = PersonEditModalPage {
        id: employee.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(EmployeeEditPostRouteTag::new(employee.id), &form_name),
        name: employee.name,
        mobile: employee.mobile,
        email: employee.email,
        show_delete: false,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<PersonForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let form_name = q.form_name();
    let post_url = modal_edit_post_url(EmployeeEditPostRouteTag::new(id), &form_name);
    match update_employee(&state.db, id, person_input_from_form(&form)).await {
        Ok(_) => respond_edit_modal_done::<ApplicantEditModalKey>(
            &htmx,
            &EmployeeDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = PersonEditModalPage {
                id,
                form_name,
                post_url,
                name: form.name,
                mobile: form.mobile,
                email: form.email,
                show_delete: false,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn terminate_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    if find_employee_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/hr/applicants").into_response();
    }
    let page = TerminateEmployeeModalPage {
        employee_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn terminate_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<TerminateEmployeeBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match terminate_employee(&state.db, id, &ctx).await {
        Ok(ex_employee_id) => respond_create_modal_done::<TerminateEmployeeModalKey>(
            &htmx,
            &q.refresh_table(),
            &ExEmployeeDetailRouteTag::new(ex_employee_id).url(),
        ),
        Err(e) => {
            let page = TerminateEmployeeModalPage {
                employee_id: id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
