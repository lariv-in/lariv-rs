use axum::{
    extract::{Multipart, Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::{CsrfToken, HtmlForm, HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_filesystem::state::FilesystemState;
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    entities::leaves::{
        leave_journal::{self, Entity as LeaveJournalEntity},
        leave_type::LeaveType,
    },
    forms::{EmployeeForm, GiveLeaveForm, TerminateEmployeeBody},
    handlers::ModalNameQuery,
    keys::{
        ApplicantEditModalKey, EmployeeCreateModalKey, EmployeeDeleteModalKey, GiveLeaveModalKey,
        LeaveJournalTableKey, TerminateEmployeeModalKey,
    },
    logic::{
        employee::{
            EmployeeWrite, create_employee, delete_employee, ensure_manager_for_superuser,
            update_employee,
        },
        ex_employee::terminate_employee,
        leave::{
            format_journal_amount, give_leave, leave_journal_balances, parse_give_leave_amount,
        },
        person::PersonInput,
        profile::{self, profile_view},
        user::user_label,
    },
    routes::{
        ApplicantHubRouteTag, EmployeeDeletePostRouteTag, EmployeeDetailRouteTag,
        EmployeeEditPostRouteTag, EmployeeLeaveJournalRouteTag, ExEmployeeDetailRouteTag,
    },
    scope::{employee_display_name, find_employee_scoped, format_timestamp},
    state::HrState,
    templates::{
        ConfirmDeletePage, EmployeeDetailPage, EmployeeFormValues, PersonCreateKind,
        PersonCreateModalPage, PersonEditModalPage, TerminateEmployeeModalPage,
        leaves::{
            EmployeeLeaveJournalPage, GiveLeaveModalPage, LeaveJournalBalance, LeaveJournalRow,
        },
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
    employee: &crate::entities::employee::Model,
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
            .map(lariv_core::datetime::format_date)
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
        verified: employee.verified,
        date_of_joining: employee
            .date_of_joining
            .map(lariv_core::datetime::format_date)
            .unwrap_or_default(),
        probation_end_date: employee
            .probation_end_date
            .map(lariv_core::datetime::format_date)
            .unwrap_or_default(),
        work_start: employee
            .work_start
            .map(lariv_core::datetime::format_time)
            .unwrap_or_default(),
        work_end: employee
            .work_end
            .map(lariv_core::datetime::format_time)
            .unwrap_or_default(),
        base_salary: money(employee.base_salary),
        hourly_wage: money(employee.hourly_wage),
        manager_id: employee
            .manager_id
            .filter(|id| *id > 0)
            .map(|id| id.to_string())
            .unwrap_or_default(),
        manager_display: user_label(db, employee.manager_id).await,
        can_edit_manager: false,
        manager_required: false,
    }
}

pub(crate) async fn employee_values_from_submit(
    db: &sea_orm::DatabaseConnection,
    submit: &<EmployeeForm as HtmlForm>::Submit,
    existing: Option<&crate::entities::employee::Model>,
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
        verified: submit.verified,
        date_of_joining: submit.date_of_joining.clone(),
        probation_end_date: submit.probation_end_date.clone(),
        work_start: submit.work_start.clone(),
        work_end: submit.work_end.clone(),
        base_salary: submit.base_salary.clone(),
        hourly_wage: submit.hourly_wage.clone(),
        manager_id: submit
            .manager_id
            .filter(|id| *id > 0)
            .map(|id| id.to_string())
            .unwrap_or_default(),
        manager_display: user_label(db, submit.manager_id).await,
        can_edit_manager: false,
        manager_required: false,
    }
}

/// Admins set a manager. The employee cannot change their own. A superuser's
/// manager is required; every other employee's manager may be empty.
async fn stamp_manager_access(
    db: &sea_orm::DatabaseConnection,
    actor_user_id: i64,
    subject_user_id: Option<i64>,
    mut values: EmployeeFormValues,
) -> EmployeeFormValues {
    let can_edit = subject_user_id.is_none_or(|id| id != actor_user_id);
    values.can_edit_manager = can_edit;
    values.manager_required = if let Some(id) = subject_user_id.filter(|_| can_edit) {
        crate::logic::user::user_is_superuser(db, id)
            .await
            .unwrap_or(false)
    } else {
        false
    };
    values
}

pub(crate) async fn employee_write_from_submit(
    fs: &FilesystemState,
    owner_id: i64,
    submit: <EmployeeForm as HtmlForm>::Submit,
    existing: Option<&crate::entities::employee::Model>,
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

#[derive(Debug, serde::Deserialize, Default)]
struct LeaveJournalQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
}

fn journal_path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn journal_query_from_uri(uri: &Uri) -> LeaveJournalQuery {
    let Some(query) = uri.query() else {
        return LeaveJournalQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn journal_sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|direction| direction.eq_ignore_ascii_case("DESC"))
}

pub async fn leave_journal(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    uri: Uri,
) -> Response {
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    };
    let q = journal_query_from_uri(&uri);
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let sort = q
        .sort
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Datetime DESC".to_string());
    let desc = journal_sort_desc(&sort);
    let mut query =
        LeaveJournalEntity::find().filter(leave_journal::Column::UserId.eq(employee.user_id));
    query = match sort.split_whitespace().next().unwrap_or("") {
        column if column.eq_ignore_ascii_case("LeaveType") => {
            if desc {
                query.order_by_desc(leave_journal::Column::LeaveType)
            } else {
                query.order_by_asc(leave_journal::Column::LeaveType)
            }
        }
        column if column.eq_ignore_ascii_case("Amount") => {
            if desc {
                query.order_by_desc(leave_journal::Column::Amount)
            } else {
                query.order_by_asc(leave_journal::Column::Amount)
            }
        }
        _ => {
            if desc {
                query.order_by_desc(leave_journal::Column::Datetime)
            } else {
                query.order_by_asc(leave_journal::Column::Datetime)
            }
        }
    };
    query = query.order_by_desc(leave_journal::Column::Id);
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|row| LeaveJournalRow {
            datetime: ctx.format_datetime(row.datetime).into_string(),
            leave_type: row.leave_type.to_string(),
            amount: format_journal_amount(row.amount),
        })
        .collect();
    let totals = leave_journal_balances(&state.db, employee.user_id).await;
    let balances = [LeaveType::Casual, LeaveType::Sick, LeaveType::Privilege]
        .into_iter()
        .zip(totals)
        .map(|(kind, amount)| LeaveJournalBalance {
            leave_type: kind.label().to_string(),
            amount: format_journal_amount(amount),
        })
        .collect();
    let page = EmployeeLeaveJournalPage {
        employee_id: employee.id,
        display_name: employee_display_name(&employee),
        balances,
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        sort,
        path_and_query: journal_path_and_query(&uri),
    };
    if htmx.targets::<LeaveJournalTableKey>() {
        return page.render_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn give_leave_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_employee_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    }
    let page = GiveLeaveModalPage::new(id, q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn give_leave_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<GiveLeaveForm>,
) -> Response {
    let Some(employee) = find_employee_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantHubRouteTag.url()).into_response();
    };
    let error = match give_leave_from_form(&form) {
        Ok((leave_type, amount)) => {
            match give_leave(&state.db, employee.user_id, leave_type, amount).await {
                Ok(()) => {
                    return respond_create_modal_done::<GiveLeaveModalKey>(
                        &htmx,
                        &q.refresh_table(),
                        &EmployeeLeaveJournalRouteTag::new(id).url(),
                    );
                }
                Err(error) => error,
            }
        }
        Err(error) => error,
    };
    let page = GiveLeaveModalPage {
        employee_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        leave_type: form.leave_type,
        amount: form.amount,
        error,
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn give_leave_from_form(form: &GiveLeaveForm) -> Result<(LeaveType, i64), String> {
    let leave_type = LeaveType::parse(form.leave_type.trim())
        .ok_or_else(|| "leave type is required".to_string())?;
    let amount = parse_give_leave_amount(&form.amount)?;
    Ok((leave_type, amount))
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
    let values = stamp_manager_access(
        &state.db,
        ctx.user.id,
        Some(employee.user_id),
        employee_values_from_model(&state.db, &employee).await,
    )
    .await;
    let page = PersonEditModalPage {
        id: employee.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(EmployeeEditPostRouteTag::new(employee.id), &form_name),
        values,
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
    let editing_self = existing.user_id == ctx.user.id;
    let submit = match EmployeeForm::from_multipart(multipart, &csrf).await {
        Ok(submit) => submit,
        Err(e) => {
            let page = PersonEditModalPage {
                id,
                form_name,
                post_url,
                values: stamp_manager_access(
                    &state.db,
                    ctx.user.id,
                    Some(existing.user_id),
                    employee_values_from_model(&state.db, &existing).await,
                )
                .await,
                show_delete: true,
                error: e.to_string(),
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let values = stamp_manager_access(
        &state.db,
        ctx.user.id,
        Some(existing.user_id),
        employee_values_from_submit(&state.db, &submit, Some(&existing)).await,
    )
    .await;
    let input = match employee_write_from_submit(&fs, ctx.user.id, submit, Some(&existing)).await {
        Ok(mut input) => {
            if editing_self {
                input.profile.manager_id = existing.manager_id;
            } else if let Err(message) =
                ensure_manager_for_superuser(&state.db, existing.user_id, input.profile.manager_id)
                    .await
            {
                let page = PersonEditModalPage {
                    id,
                    form_name,
                    post_url,
                    values,
                    show_delete: true,
                    error: message,
                };
                return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response();
            }
            input
        }
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
            &lariv_core::http::RouteQueryBuilder::new(ApplicantHubRouteTag)
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
