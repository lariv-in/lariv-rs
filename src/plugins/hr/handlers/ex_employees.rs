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
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
    },
};

use crate::plugins::hr::{
    forms::ApplicantForm,
    handlers::{ModalNameQuery, applicants::person_input_from_form},
    keys::ExEmployeeCreateModalKey,
    logic::ex_employee::create_ex_employee,
    routes::ExEmployeeDetailRouteTag,
    scope::{ex_employee_display_name, find_ex_employee_scoped, format_timestamp},
    state::HrState,
    templates::{ExEmployeeDetailPage, PersonCreateKind, PersonCreateModalPage},
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
        "New ex-employee",
        "Create ex-employee",
        PersonCreateKind::ExEmployee,
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ApplicantForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match create_ex_employee(&state.db, person_input_from_form(&form)).await {
        Ok(ex_employee) => respond_create_modal_done::<ExEmployeeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &ExEmployeeDetailRouteTag::new(ex_employee.id).url(),
        ),
        Err(e) => {
            let page = PersonCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                "New ex-employee",
                "Create ex-employee",
                PersonCreateKind::ExEmployee,
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
    let Some(ex_employee) = find_ex_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let page = ExEmployeeDetailPage {
        id: ex_employee.id,
        display_name: ex_employee_display_name(&ex_employee),
        name: ex_employee.name,
        mobile: ex_employee.mobile,
        email: ex_employee.email,
        terminated_at: format_timestamp(ex_employee.terminated_at, &ctx.timezone),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
