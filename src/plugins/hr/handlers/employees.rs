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
    forms::{EmployeeForm, TerminateEmployeeBody},
    handlers::ModalNameQuery,
    keys::{ApplicantEditModalKey, EmployeeCreateModalKey, TerminateEmployeeModalKey},
    logic::{
        employee::{EmployeeWrite, create_employee, update_employee},
        ex_employee::terminate_employee,
        person::PersonInput,
        profile::{self, profile_from_form, profile_view},
    },
    routes::{
        ApplicantHubRouteTag, EmployeeDetailRouteTag, EmployeeEditPostRouteTag,
        ExEmployeeDetailRouteTag,
    },
    scope::{employee_display_name, find_employee_scoped, format_timestamp},
    state::HrState,
    templates::{
        EmployeeDetailPage, EmployeeFormValues, PersonCreateKind, PersonCreateModalPage,
        PersonEditModalPage, TerminateEmployeeModalPage,
    },
};

pub(crate) async fn employee_values_from_model(
    db: &sea_orm::DatabaseConnection,
    employee: &crate::plugins::hr::entities::employee::Model,
) -> EmployeeFormValues {
    let (photograph_href, photograph_display) =
        profile::vnode_view(db, employee.photograph_vnode_id).await;
    let _ = photograph_href;
    let (_, aadhar_display) = profile::document_view(
        db,
        employee.aadhar_document_id,
        crate::plugins::documents::document_type::DocumentType::AadharCard,
    )
    .await;
    let (_, pan_display) = profile::document_view(
        db,
        employee.pan_document_id,
        crate::plugins::documents::document_type::DocumentType::Pan,
    )
    .await;
    let (_, passport_display) = profile::document_view(
        db,
        employee.passport_document_id,
        crate::plugins::documents::document_type::DocumentType::Passport,
    )
    .await;
    EmployeeFormValues {
        name: employee.name.clone(),
        mobile: employee.mobile.clone(),
        email: employee.email.clone(),
        fathers_name: employee.fathers_name.clone(),
        date_of_birth: employee
            .date_of_birth
            .map(crate::datetime::format_date)
            .unwrap_or_default(),
        gender: employee
            .gender
            .map(|gender| gender.as_str().to_string())
            .unwrap_or_default(),
        marital_status: employee.marital_status.clone(),
        nationality: employee.nationality.clone(),
        is_disabled: employee.is_disabled,
        disability_type: employee.disability_type.clone(),
        photograph_vnode_id: fk_string(employee.photograph_vnode_id),
        photograph_display,
        blood_group: employee
            .blood_group
            .map(|group| group.as_str().to_string())
            .unwrap_or_default(),
        identification_mark: employee.identification_mark.clone(),
        present_address: employee.present_address.clone(),
        present_pin_code: employee.present_pin_code.clone(),
        permanent_address: employee.permanent_address.clone(),
        permanent_pin_code: employee.permanent_pin_code.clone(),
        emergency_contact_name: employee.emergency_contact_name.clone(),
        emergency_contact_relation: employee.emergency_contact_relation.clone(),
        emergency_contact_mobile: employee.emergency_contact_mobile.clone(),
        aadhar_document_id: fk_string(employee.aadhar_document_id),
        aadhar_display,
        pan_document_id: fk_string(employee.pan_document_id),
        pan_display,
        passport_document_id: fk_string(employee.passport_document_id),
        passport_display,
        account_holder_name: employee.account_holder_name.clone(),
        account_number: employee.account_number.clone(),
        account_ifsc_code: employee.account_ifsc_code.clone(),
        account_type: employee.account_type.clone(),
        qualifications: employee.qualifications.clone(),
        date_of_joining: employee
            .date_of_joining
            .map(crate::datetime::format_date)
            .unwrap_or_default(),
        probation_end_date: employee
            .probation_end_date
            .map(crate::datetime::format_date)
            .unwrap_or_default(),
    }
}

pub(crate) async fn employee_values_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &EmployeeForm,
) -> EmployeeFormValues {
    let photograph_id = crate::plugins::hr::logic::applicant::parse_optional_fk(&form.photograph_vnode_id);
    let (_, photograph_display) = profile::vnode_view(db, photograph_id).await;
    let aadhar_id = crate::plugins::hr::logic::applicant::parse_optional_fk(&form.aadhar_document_id);
    let (_, aadhar_display) = profile::document_view(
        db,
        aadhar_id,
        crate::plugins::documents::document_type::DocumentType::AadharCard,
    )
    .await;
    let pan_id = crate::plugins::hr::logic::applicant::parse_optional_fk(&form.pan_document_id);
    let (_, pan_display) = profile::document_view(
        db,
        pan_id,
        crate::plugins::documents::document_type::DocumentType::Pan,
    )
    .await;
    let passport_id =
        crate::plugins::hr::logic::applicant::parse_optional_fk(&form.passport_document_id);
    let (_, passport_display) = profile::document_view(
        db,
        passport_id,
        crate::plugins::documents::document_type::DocumentType::Passport,
    )
    .await;
    EmployeeFormValues {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
        fathers_name: form.fathers_name.clone(),
        date_of_birth: form.date_of_birth.clone(),
        gender: form.gender.clone(),
        marital_status: form.marital_status.clone(),
        nationality: form.nationality.clone(),
        is_disabled: form.is_disabled,
        disability_type: form.disability_type.clone(),
        photograph_vnode_id: form.photograph_vnode_id.clone(),
        photograph_display,
        blood_group: form.blood_group.clone(),
        identification_mark: form.identification_mark.clone(),
        present_address: form.present_address.clone(),
        present_pin_code: form.present_pin_code.clone(),
        permanent_address: form.permanent_address.clone(),
        permanent_pin_code: form.permanent_pin_code.clone(),
        emergency_contact_name: form.emergency_contact_name.clone(),
        emergency_contact_relation: form.emergency_contact_relation.clone(),
        emergency_contact_mobile: form.emergency_contact_mobile.clone(),
        aadhar_document_id: form.aadhar_document_id.clone(),
        aadhar_display,
        pan_document_id: form.pan_document_id.clone(),
        pan_display,
        passport_document_id: form.passport_document_id.clone(),
        passport_display,
        account_holder_name: form.account_holder_name.clone(),
        account_number: form.account_number.clone(),
        account_ifsc_code: form.account_ifsc_code.clone(),
        account_type: form.account_type.clone(),
        qualifications: form.qualifications.clone(),
        date_of_joining: form.date_of_joining.clone(),
        probation_end_date: form.probation_end_date.clone(),
    }
}

pub(crate) async fn employee_write_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &EmployeeForm,
) -> Result<EmployeeWrite, String> {
    Ok(EmployeeWrite {
        person: PersonInput {
            name: form.name.clone(),
            mobile: form.mobile.clone(),
            email: form.email.clone(),
        },
        profile: profile_from_form(db, form).await?,
    })
}

fn fk_string(id: Option<i64>) -> String {
    id.filter(|id| *id > 0)
        .map(|id| id.to_string())
        .unwrap_or_default()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
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
    HtmlFormBody(form): HtmlFormBody<EmployeeForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    }
    let input = match employee_write_from_form(&state.db, &form).await {
        Ok(input) => input,
        Err(e) => {
            let page = PersonCreateModalPage::with_employee(
                q.form_name(),
                q.refresh_table(),
                "New employee",
                "Create employee",
                PersonCreateKind::Employee,
                employee_values_from_form(&state.db, &form).await,
                e,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match create_employee(&state.db, input).await {
        Ok(employee) => respond_create_modal_done::<EmployeeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &EmployeeDetailRouteTag::new(employee.id).url(),
        ),
        Err(e) => {
            let page = PersonCreateModalPage::with_employee(
                q.form_name(),
                q.refresh_table(),
                "New employee",
                "Create employee",
                PersonCreateKind::Employee,
                employee_values_from_form(&state.db, &form).await,
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
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    };
    let can_edit = ctx.user.is_superuser;
    let profile = profile_view(&state.db, &employee).await;
    let page = EmployeeDetailPage {
        id: employee.id,
        display_name: employee_display_name(&employee),
        name: employee.name,
        mobile: employee.mobile,
        email: employee.email,
        hired_at: format_timestamp(employee.hired_at, &ctx.timezone),
        is_probationary: employee.is_probationary,
        profile,
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
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    }
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    };
    let form_name = q.form_name();
    let page = PersonEditModalPage {
        id: employee.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(EmployeeEditPostRouteTag::new(employee.id), &form_name),
        values: employee_values_from_model(&state.db, &employee).await,
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
    HtmlFormBody(form): HtmlFormBody<EmployeeForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    }
    let form_name = q.form_name();
    let post_url = modal_edit_post_url(EmployeeEditPostRouteTag::new(id), &form_name);
    let input = match employee_write_from_form(&state.db, &form).await {
        Ok(input) => input,
        Err(e) => {
            let page = PersonEditModalPage {
                id,
                form_name,
                post_url,
                values: employee_values_from_form(&state.db, &form).await,
                show_delete: false,
                error: e,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match update_employee(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<ApplicantEditModalKey>(
            &htmx,
            &EmployeeDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = PersonEditModalPage {
                id,
                form_name,
                post_url,
                values: employee_values_from_form(&state.db, &form).await,
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
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    }
    if find_employee_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
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
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
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
