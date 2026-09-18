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
        html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done,
        respond_edit_modal_done, modal_edit_post_url, Htmx,
    },
};

use crate::plugins::hr::{
    forms::{ApplicantForm, HireEmployeeBody},
    handlers::{applicants::person_input_from_form, ModalNameQuery},
    keys::{ApplicantEditModalKey, HireEmployeeModalKey, ProbationCreateModalKey},
    logic::{
        employee::hire_employee,
        probation::{create_probation, update_probation},
    },
    routes::{EmployeeDetailRouteTag, ProbationDetailRouteTag, ProbationEditPostRouteTag},
    scope::{find_probation_scoped, format_timestamp, probation_display_name},
    state::HrState,
    templates::{
        HireEmployeeModalPage, PersonCreateKind, PersonCreateModalPage, PersonEditModalPage,
        ProbationDetailPage,
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
        "New probation",
        "Create probation",
        PersonCreateKind::Probation,
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
    match create_probation(&state.db, person_input_from_form(&form)).await {
        Ok(probation) => respond_create_modal_done::<ProbationCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &ProbationDetailRouteTag::new(probation.id).url(),
        ),
        Err(e) => {
            let page = PersonCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                "New probation",
                "Create probation",
                PersonCreateKind::Probation,
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
    let Some(probation) = find_probation_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let can_edit = ctx.user.is_superuser;
    let page = ProbationDetailPage {
        id: probation.id,
        display_name: probation_display_name(&probation),
        name: probation.name,
        mobile: probation.mobile,
        email: probation.email,
        started_at: format_timestamp(probation.started_at, &ctx.timezone),
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
    let Some(probation) = find_probation_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let form_name = q.form_name();
    let page = PersonEditModalPage {
        id: probation.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(ProbationEditPostRouteTag::new(probation.id), &form_name),
        name: probation.name,
        mobile: probation.mobile,
        email: probation.email,
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
    HtmlFormBody(form): HtmlFormBody<ApplicantForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let form_name = q.form_name();
    let post_url = modal_edit_post_url(ProbationEditPostRouteTag::new(id), &form_name);
    match update_probation(&state.db, id, person_input_from_form(&form)).await {
        Ok(_) => respond_edit_modal_done::<ApplicantEditModalKey>(
            &htmx,
            &ProbationDetailRouteTag::new(id).url(),
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

pub async fn hire_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    if find_probation_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/hr/applicants").into_response();
    }
    let page = HireEmployeeModalPage {
        probation_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn hire_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<HireEmployeeBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match hire_employee(&state.db, id, &ctx).await {
        Ok(employee_id) => respond_create_modal_done::<HireEmployeeModalKey>(
            &htmx,
            &q.refresh_table(),
            &EmployeeDetailRouteTag::new(employee_id).url(),
        ),
        Err(e) => {
            let page = HireEmployeeModalPage {
                probation_id: id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
