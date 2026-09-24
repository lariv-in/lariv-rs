use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{EntityTrait, PaginatorTrait};

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::{HtmlFormBody, UrlencodedFields},
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::hr::{
    entities::{
        applicant::{self, Entity as ApplicantEntity},
        employee::Entity as EmployeeEntity,
        ex_employee::Entity as ExEmployeeEntity,
        job_form::Entity as JobFormEntity,
        probation::Entity as ProbationEntity,
    },
    forms::{ApplicantForm, PersonForm, StartProbationBody},
    handlers::ModalNameQuery,
    keys::{
        ApplicantCreateModalKey, ApplicantDeleteModalKey, ApplicantEditModalKey,
        ApplicantHubTableKey, StartProbationModalKey,
    },
    logic::{
        applicant::{
            ApplicantInput, create_applicant, delete_applicant, parse_optional_age,
            parse_optional_fk, parse_optional_gender, parse_optional_text, update_applicant,
        },
        person::PersonInput,
        probation::start_probation,
    },
    routes::{
        ApplicantDetailRouteTag, ApplicantEditPostRouteTag, ApplicantHubRouteTag,
        ProbationDetailRouteTag,
    },
    scope::{
        applicant_display_name, apply_applicant_filters, apply_applicant_sort,
        apply_employee_filters, apply_employee_sort, apply_ex_employee_filters,
        apply_ex_employee_sort, apply_probation_filters, apply_probation_sort,
        employee_display_name, ex_employee_display_name, find_applicant_scoped,
        probation_display_name, scope_applicants, scope_employees, scope_ex_employees,
        scope_probations,
    },
    state::HrState,
    templates::{
        ApplicantCreateModalPage, ApplicantDetailPage, ApplicantEditModalPage, ApplicantFormValues,
        ApplicantHubPage, ApplicantRow, ConfirmDeletePage, StartProbationModalPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct HubQuery {
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "Email", alias = "email")]
    pub email: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
}

fn hub_query_from_uri(uri: &Uri) -> HubQuery {
    let Some(query) = uri.query() else {
        return HubQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub(crate) fn person_input_from_form(form: &PersonForm) -> PersonInput {
    PersonInput {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
    }
}

fn applicant_input_from_form(form: &ApplicantForm) -> Result<ApplicantInput, String> {
    Ok(ApplicantInput {
        person: PersonInput {
            name: form.name.clone(),
            mobile: form.mobile.clone(),
            email: form.email.clone(),
        },
        form_response_id: parse_optional_fk(&form.form_response_id),
        age: parse_optional_age(&form.age)?,
        gender: parse_optional_gender(&form.gender)?,
        resume_vnode_id: parse_optional_fk(&form.resume_vnode_id),
        job_form_id: parse_optional_fk(&form.job_form_id),
        remarks: parse_optional_text(&form.remarks),
        address: parse_optional_text(&form.address),
    })
}

fn fk_string(id: Option<i64>) -> String {
    id.filter(|id| *id > 0)
        .map(|id| id.to_string())
        .unwrap_or_default()
}

async fn job_form_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    crate::web::opt_or_log(JobFormEntity::find_by_id(id).one(db).await, "find job form")
        .map(|j| j.job_title)
        .unwrap_or_else(|| format!("Job posting #{id}"))
}

async fn form_response_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    use crate::plugins::forms::entities::form_response::Entity as FormResponseEntity;
    let Some(r) = crate::web::opt_or_log(
        FormResponseEntity::find_by_id(id).one(db).await,
        "find form response",
    ) else {
        return format!("Response #{id}");
    };
    match (
        r.name.as_deref().unwrap_or("").trim(),
        r.email.as_deref().unwrap_or("").trim(),
    ) {
        ("", "") => format!("Response #{id}"),
        ("", email) => email.to_string(),
        (name, "") => name.to_string(),
        (name, email) => format!("{name} ({email})"),
    }
}

async fn resume_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    use crate::plugins::filesystem::entities::filesystem_node::Entity as VNodeEntity;
    crate::web::opt_or_log(
        VNodeEntity::find_by_id(id).one(db).await,
        "find resume vnode",
    )
    .map(|n| n.name)
    .unwrap_or_else(|| format!("File #{id}"))
}

async fn form_values_from_applicant(
    db: &sea_orm::DatabaseConnection,
    applicant: &applicant::Model,
) -> ApplicantFormValues {
    ApplicantFormValues {
        name: applicant.name.clone(),
        mobile: applicant.mobile.clone(),
        email: applicant.email.clone(),
        age: applicant
            .age
            .filter(|n| *n > 0)
            .map(crate::duration::format_duration)
            .unwrap_or_default(),
        gender: applicant
            .gender
            .map(|g| g.as_str().to_string())
            .unwrap_or_default(),
        address: applicant.address.clone().unwrap_or_default(),
        remarks: applicant.remarks.clone().unwrap_or_default(),
        job_form_id: fk_string(applicant.job_form_id),
        job_form_display: job_form_display(db, applicant.job_form_id).await,
        form_response_id: fk_string(applicant.form_response_id),
        form_response_display: form_response_display(db, applicant.form_response_id).await,
        resume_vnode_id: fk_string(applicant.resume_vnode_id),
        resume_display: resume_display(db, applicant.resume_vnode_id).await,
    }
}

async fn form_values_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ApplicantForm,
) -> ApplicantFormValues {
    ApplicantFormValues {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
        age: form.age.clone(),
        gender: form.gender.clone(),
        address: form.address.clone(),
        remarks: form.remarks.clone(),
        job_form_id: form.job_form_id.clone(),
        job_form_display: job_form_display(db, parse_optional_fk(&form.job_form_id)).await,
        form_response_id: form.form_response_id.clone(),
        form_response_display: form_response_display(db, parse_optional_fk(&form.form_response_id))
            .await,
        resume_vnode_id: form.resume_vnode_id.clone(),
        resume_display: resume_display(db, parse_optional_fk(&form.resume_vnode_id)).await,
    }
}

fn person_row(
    id: i64,
    name: String,
    mobile: String,
    email: String,
    status: &str,
    detail_href: String,
) -> ApplicantRow {
    ApplicantRow {
        id,
        name,
        mobile,
        email,
        status: status.to_string(),
        detail_href,
    }
}

pub(crate) async fn query_applicants(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = scope_applicants(ApplicantEntity::find(), auth);
    query = apply_applicant_filters(query, q.name.as_deref(), q.email.as_deref());
    query = apply_applicant_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|a| {
            person_row(
                a.id,
                applicant_display_name(&a),
                a.mobile,
                a.email,
                "Applicant",
                ApplicantDetailRouteTag::new(a.id).url(),
            )
        })
        .collect();
    (rows, page_num, total)
}

pub(crate) async fn query_probations(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = scope_probations(ProbationEntity::find(), auth);
    query = apply_probation_filters(query, q.name.as_deref(), q.email.as_deref());
    query = apply_probation_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|p| {
            person_row(
                p.id,
                probation_display_name(&p),
                p.mobile,
                p.email,
                "Probation",
                ProbationDetailRouteTag::new(p.id).url(),
            )
        })
        .collect();
    (rows, page_num, total)
}

pub(crate) async fn query_employees(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = scope_employees(EmployeeEntity::find(), auth);
    query = apply_employee_filters(query, q.name.as_deref(), q.email.as_deref());
    query = apply_employee_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|e| {
            person_row(
                e.id,
                employee_display_name(&e),
                e.mobile,
                e.email,
                "Employee",
                crate::plugins::hr::routes::EmployeeDetailRouteTag::new(e.id).url(),
            )
        })
        .collect();
    (rows, page_num, total)
}

pub(crate) async fn query_ex_employees(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &HubQuery,
    page_size: u32,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.unwrap_or(1).max(1);
    let mut query = scope_ex_employees(ExEmployeeEntity::find(), auth);
    query = apply_ex_employee_filters(query, q.name.as_deref(), q.email.as_deref());
    query = apply_ex_employee_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|x| {
            person_row(
                x.id,
                ex_employee_display_name(&x),
                x.mobile,
                x.email,
                "Ex-employee",
                crate::plugins::hr::routes::ExEmployeeDetailRouteTag::new(x.id).url(),
            )
        })
        .collect();
    (rows, page_num, total)
}

pub async fn hub(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let q = hub_query_from_uri(&uri);
    let tab = q.tab.as_deref().unwrap_or("applicants").to_string();
    let (rows, page, total) = match tab.as_str() {
        "probation" => query_probations(&state.db, &ctx, &q, q.page_size.get()).await,
        "employees" => query_employees(&state.db, &ctx, &q, q.page_size.get()).await,
        "ex_employees" => query_ex_employees(&state.db, &ctx, &q, q.page_size.get()).await,
        _ => query_applicants(&state.db, &ctx, &q, q.page_size.get()).await,
    };
    let people = ObjectList::from_page(rows, page, q.page_size.get(), total);
    let page = ApplicantHubPage {
        people,
        tab,
        filter_name: q.name.clone().unwrap_or_default(),
        filter_email: q.email.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<ApplicantHubTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &slot_ctx)
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = ApplicantCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        values: ApplicantFormValues::default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
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
    match applicant_input_from_form(&form) {
        Ok(input) => match create_applicant(&state.db, input).await {
            Ok(applicant) => respond_create_modal_done::<ApplicantCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &ApplicantDetailRouteTag::new(applicant.id).url(),
            ),
            Err(e) => {
                let page = ApplicantCreateModalPage {
                    form_name: q.form_name(),
                    refresh_table: q.refresh_table(),
                    values: form_values_from_form(&state.db, &form).await,
                    error: e,
                };
                html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response()
            }
        },
        Err(e) => {
            let page = ApplicantCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                values: form_values_from_form(&state.db, &form).await,
                error: e,
            };
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
    let Some(applicant) = find_applicant_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let can_edit = ctx.user.is_superuser;
    let values = form_values_from_applicant(&state.db, &applicant).await;
    let page = ApplicantDetailPage {
        id: applicant.id,
        display_name: applicant_display_name(&applicant),
        values,
        job_form_href: applicant
            .job_form_id
            .filter(|id| *id > 0)
            .map(|id| crate::plugins::hr::routes::JobFormDetailRouteTag::new(id).url())
            .unwrap_or_default(),
        form_response_href: applicant
            .form_response_id
            .filter(|id| *id > 0)
            .map(|id| crate::plugins::forms::routes::FormResponseDetailRouteTag::new(id).url())
            .unwrap_or_default(),
        resume_href: applicant
            .resume_vnode_id
            .filter(|id| *id > 0)
            .map(|id| crate::plugins::filesystem::routes::VNodeDetailRouteTag::new(id).url())
            .unwrap_or_default(),
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
    let Some(applicant) = find_applicant_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/applicants").into_response();
    };
    let form_name = q.form_name();
    let page = ApplicantEditModalPage {
        id: applicant.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(ApplicantEditPostRouteTag::new(applicant.id), &form_name),
        values: form_values_from_applicant(&state.db, &applicant).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn applicant_edit_modal_error(
    id: i64,
    post_url: String,
    q: &ModalNameQuery,
    form: &ApplicantForm,
    error: &str,
    chrome: &SharedChromeFolder,
    ctx: &crate::plugins::users::state::AuthContext,
    db: &sea_orm::DatabaseConnection,
) -> Response {
    let page = ApplicantEditModalPage {
        id,
        form_name: q.form_name(),
        post_url,
        values: form_values_from_form(db, form).await,
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
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
    let post_url = modal_edit_post_url(ApplicantEditPostRouteTag::new(id), &form_name);
    match applicant_input_from_form(&form) {
        Ok(input) => match update_applicant(&state.db, id, input).await {
            Ok(_) => respond_edit_modal_done::<ApplicantEditModalKey>(
                &htmx,
                &ApplicantDetailRouteTag::new(id).url(),
            ),
            Err(e) => {
                applicant_edit_modal_error(id, post_url, &q, &form, &e, &chrome, &ctx, &state.db)
                    .await
            }
        },
        Err(e) => {
            applicant_edit_modal_error(id, post_url, &q, &form, &e, &chrome, &ctx, &state.db).await
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: ApplicantDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this applicant?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_hr.ApplicantDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match delete_applicant(&state.db, id).await {
        Ok(()) => htmx.redirect(&ApplicantHubRouteTag.url()),
        Err(e) => {
            let page = ConfirmDeletePage {
                modal_uid: ApplicantDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this applicant?".into(),
                form_name: "p_hr.ApplicantDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn start_probation_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    if find_applicant_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/hr/applicants").into_response();
    }
    let page = StartProbationModalPage {
        applicant_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn start_probation_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<StartProbationBody>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match start_probation(&state.db, id, &ctx).await {
        Ok(probation_id) => respond_create_modal_done::<StartProbationModalKey>(
            &htmx,
            &q.refresh_table(),
            &ProbationDetailRouteTag::new(probation_id).url(),
        ),
        Err(e) => {
            let page = StartProbationModalPage {
                applicant_id: id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
