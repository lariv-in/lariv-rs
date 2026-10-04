use axum::{
    extract::{Multipart, Path, Query},
    response::{IntoResponse, Redirect, Response},
};

use crate::{
    components::{SharedChromeFolder, SlotCtx, SwapKey},
    html_form::{CsrfToken, HtmlForm, HtmlFormBody},
    http::Cap,
    plugins::{filesystem::state::FilesystemState, users::middleware::RequireAuth},
    web::{
        Htmx, html_built_page_or_app_layout, html_built_page_with_slots, modal_edit_post_url,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::hr::{
    forms::{EmployeeForm, TerminateEmployeeBody},
    handlers::ModalNameQuery,
    keys::{
        ApplicantEditModalKey, EmployeeCreateModalKey, EmployeeDeleteModalKey,
        TerminateEmployeeModalKey,
    },
    logic::{
        employee::{EmployeeWrite, create_employee, delete_employee, update_employee},
        ex_employee::terminate_employee,
        person::PersonInput,
        profile::{self, profile_view},
    },
    routes::{
        ApplicantHubRouteTag, EmployeeDeletePostRouteTag, EmployeeDetailRouteTag,
        EmployeeEditPostRouteTag, ExEmployeeDetailRouteTag,
    },
    scope::{employee_display_name, find_employee_scoped, format_timestamp},
    state::HrState,
    templates::{
        ConfirmDeletePage, EmployeeDetailPage, EmployeeFormValues, PersonCreateKind,
        PersonCreateModalPage, PersonEditModalPage, TerminateEmployeeModalPage,
    },
};

fn text(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn money(value: Option<rust_decimal::Decimal>) -> String {
    value
        .map(|amount| amount.normalize().to_string())
        .unwrap_or_default()
}

pub(crate) async fn employee_values_from_model(
    db: &sea_orm::DatabaseConnection,
    employee: &crate::plugins::hr::entities::employee::Model,
) -> EmployeeFormValues {
    let (_, photograph_display) = profile::vnode_view(db, employee.photograph_vnode_id).await;
    let (_, aadhar_display) = profile::vnode_view(db, employee.aadhar_vnode_id).await;
    let (_, pan_display) = profile::vnode_view(db, employee.pan_vnode_id).await;
    let (_, passport_display) = profile::vnode_view(db, employee.passport_vnode_id).await;
    let present_address = text(&employee.present_address);
    let present_pin_code = text(&employee.present_pin_code);
    let permanent_address = text(&employee.permanent_address);
    let permanent_pin_code = text(&employee.permanent_pin_code);
    let same_as_present = !present_address.is_empty()
        && present_address == permanent_address
        && present_pin_code == permanent_pin_code;
    EmployeeFormValues {
        name: text(&employee.name),
        mobile: text(&employee.mobile),
        email: text(&employee.email),
        fathers_name: text(&employee.fathers_name),
        date_of_birth: employee
            .date_of_birth
            .map(crate::datetime::format_date)
            .unwrap_or_default(),
        gender: employee
            .gender
            .map(|gender| gender.as_str().to_string())
            .unwrap_or_default(),
        marital_status: text(&employee.marital_status),
        nationality: text(&employee.nationality),
        is_disabled: employee.is_disabled.unwrap_or(false),
        disability_type: text(&employee.disability_type),
        photograph_display,
        blood_group: employee
            .blood_group
            .map(|group| group.as_str().to_string())
            .unwrap_or_default(),
        identification_mark: text(&employee.identification_mark),
        present_address,
        present_pin_code,
        same_as_present,
        permanent_address,
        permanent_pin_code,
        emergency_contact_name: text(&employee.emergency_contact_name),
        emergency_contact_relation: text(&employee.emergency_contact_relation),
        emergency_contact_mobile: text(&employee.emergency_contact_mobile),
        aadhar_display,
        pan_display,
        passport_display,
        account_holder_name: text(&employee.account_holder_name),
        account_number: text(&employee.account_number),
        account_ifsc_code: text(&employee.account_ifsc_code),
        account_type: text(&employee.account_type),
        qualifications: text(&employee.qualifications),
        date_of_joining: employee
            .date_of_joining
            .map(crate::datetime::format_date)
            .unwrap_or_default(),
        probation_end_date: employee
            .probation_end_date
            .map(crate::datetime::format_date)
            .unwrap_or_default(),
        work_start: employee
            .work_start
            .map(crate::datetime::format_time)
            .unwrap_or_default(),
        work_end: employee
            .work_end
            .map(crate::datetime::format_time)
            .unwrap_or_default(),
        base_salary: money(employee.base_salary),
        hourly_wage: money(employee.hourly_wage),
    }
}

pub(crate) async fn employee_values_from_submit(
    db: &sea_orm::DatabaseConnection,
    submit: &<EmployeeForm as HtmlForm>::Submit,
    existing: Option<&crate::plugins::hr::entities::employee::Model>,
) -> EmployeeFormValues {
    let photograph_display = if let Some(file) = &submit.photograph {
        file.filename().to_string()
    } else if let Some(e) = existing {
        profile::vnode_view(db, e.photograph_vnode_id).await.1
    } else {
        String::new()
    };
    let aadhar_display = if let Some(file) = &submit.aadhar {
        file.filename().to_string()
    } else if let Some(e) = existing {
        profile::vnode_view(db, e.aadhar_vnode_id).await.1
    } else {
        String::new()
    };
    let pan_display = if let Some(file) = &submit.pan {
        file.filename().to_string()
    } else if let Some(e) = existing {
        profile::vnode_view(db, e.pan_vnode_id).await.1
    } else {
        String::new()
    };
    let passport_display = if let Some(file) = &submit.passport {
        file.filename().to_string()
    } else if let Some(e) = existing {
        profile::vnode_view(db, e.passport_vnode_id).await.1
    } else {
        String::new()
    };
    let (permanent_address, permanent_pin_code) = if submit.same_as_present {
        (
            submit.present_address.clone(),
            submit.present_pin_code.clone(),
        )
    } else {
        (
            submit.permanent_address.clone(),
            submit.permanent_pin_code.clone(),
        )
    };
    EmployeeFormValues {
        name: submit.name.clone(),
        mobile: submit.mobile.clone(),
        email: submit.email.clone(),
        fathers_name: submit.fathers_name.clone(),
        date_of_birth: submit.date_of_birth.clone(),
        gender: submit.gender.clone(),
        marital_status: submit.marital_status.clone(),
        nationality: submit.nationality.clone(),
        is_disabled: submit.is_disabled,
        disability_type: submit.disability_type.clone(),
        photograph_display,
        blood_group: submit.blood_group.clone(),
        identification_mark: submit.identification_mark.clone(),
        present_address: submit.present_address.clone(),
        present_pin_code: submit.present_pin_code.clone(),
        same_as_present: submit.same_as_present,
        permanent_address,
        permanent_pin_code,
        emergency_contact_name: submit.emergency_contact_name.clone(),
        emergency_contact_relation: submit.emergency_contact_relation.clone(),
        emergency_contact_mobile: submit.emergency_contact_mobile.clone(),
        aadhar_display,
        pan_display,
        passport_display,
        account_holder_name: submit.account_holder_name.clone(),
        account_number: submit.account_number.clone(),
        account_ifsc_code: submit.account_ifsc_code.clone(),
        account_type: submit.account_type.clone(),
        qualifications: submit.qualifications.clone(),
        date_of_joining: submit.date_of_joining.clone(),
        probation_end_date: submit.probation_end_date.clone(),
        work_start: submit.work_start.clone(),
        work_end: submit.work_end.clone(),
        base_salary: submit.base_salary.clone(),
        hourly_wage: submit.hourly_wage.clone(),
    }
}

pub(crate) async fn employee_write_from_submit(
    fs: &FilesystemState,
    owner_id: i64,
    submit: <EmployeeForm as HtmlForm>::Submit,
    existing: Option<&crate::plugins::hr::entities::employee::Model>,
) -> Result<EmployeeWrite, String> {
    let name = submit.name.clone();
    let mobile = submit.mobile.clone();
    let email = submit.email.clone();
    Ok(EmployeeWrite {
        person: PersonInput {
            name: name.clone(),
            mobile,
            email,
        },
        profile: profile::profile_from_submit(fs, owner_id, &name, submit, existing).await?,
    })
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
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
    Cap(fs): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    let submit = match EmployeeForm::from_multipart(multipart, &csrf).await {
        Ok(submit) => submit,
        Err(e) => {
            let page = PersonCreateModalPage::with_employee(
                q.form_name(),
                q.refresh_table(),
                "New employee",
                "Create employee",
                PersonCreateKind::Employee,
                EmployeeFormValues::default(),
                e.to_string(),
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let values = employee_values_from_submit(&state.db, &submit, None).await;
    let input = match employee_write_from_submit(&fs, ctx.user.id, submit, None).await {
        Ok(input) => input,
        Err(e) => {
            let page = PersonCreateModalPage::with_employee(
                q.form_name(),
                q.refresh_table(),
                "New employee",
                "Create employee",
                PersonCreateKind::Employee,
                values,
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
                values,
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
    let profile = profile_view(&state.db, &employee).await;
    let page = EmployeeDetailPage {
        id: employee.id,
        display_name: employee_display_name(&employee),
        name: text(&employee.name),
        mobile: text(&employee.mobile),
        email: text(&employee.email),
        hired_at: format_timestamp(employee.hired_at, &ctx.timezone),
        is_probationary: employee.is_probationary,
        profile,
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
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    };
    let form_name = q.form_name();
    let page = PersonEditModalPage {
        id: employee.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(EmployeeEditPostRouteTag::new(employee.id), &form_name),
        values: employee_values_from_model(&state.db, &employee).await,
        show_delete: true,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<HrState>,
    Cap(fs): Cap<FilesystemState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    csrf: CsrfToken,
    multipart: Multipart,
) -> Response {
    let Some(existing) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    };
    let form_name = q.form_name();
    let post_url = modal_edit_post_url(EmployeeEditPostRouteTag::new(id), &form_name);
    let submit = match EmployeeForm::from_multipart(multipart, &csrf).await {
        Ok(submit) => submit,
        Err(e) => {
            let page = PersonEditModalPage {
                id,
                form_name,
                post_url,
                values: employee_values_from_model(&state.db, &existing).await,
                show_delete: true,
                error: e.to_string(),
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let values = employee_values_from_submit(&state.db, &submit, Some(&existing)).await;
    let input = match employee_write_from_submit(&fs, ctx.user.id, submit, Some(&existing)).await {
        Ok(input) => input,
        Err(e) => {
            let page = PersonEditModalPage {
                id,
                form_name,
                post_url,
                values,
                show_delete: true,
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
                values,
                show_delete: true,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_employee_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    }
    let page = ConfirmDeletePage {
        modal_uid: EmployeeDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this employee record?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_hr.EmployeeDeleteForm".into()),
        id,
        post_url: EmployeeDeletePostRouteTag::new(id).url(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match delete_employee(&state.db, id).await {
        Ok(()) => htmx.redirect(
            &crate::http::RouteQueryBuilder::new(ApplicantHubRouteTag)
                .query("tab", "employees")
                .build(),
        ),
        Err(e) => {
            let page = ConfirmDeletePage {
                modal_uid: EmployeeDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this employee record?".into(),
                form_name: "p_hr.EmployeeDeleteForm".into(),
                id,
                post_url: EmployeeDeletePostRouteTag::new(id).url(),
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
