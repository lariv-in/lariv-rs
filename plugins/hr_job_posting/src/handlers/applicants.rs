use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::sea_query::Condition;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use crate::{
    entities::{
        applicant::{self, Entity as ApplicantEntity},
        job_form::Entity as JobFormEntity,
    },
    forms::{ApplicantForm, HireApplicantBody},
    handlers::ModalNameQuery,
    keys::{APPLICANT_CREATE_MODAL, APPLICANT_EDIT_MODAL, APPLICANT_TABLE, HIRE_APPLICANT_MODAL},
    logic::applicant::{
        ApplicantInput, create_applicant, delete_applicant, hire_applicant,
        parse_optional_datetime, parse_optional_fk, parse_optional_gender, parse_optional_text,
        update_applicant,
    },
    routes::{ApplicantDetailRouteTag, ApplicantEditPostRouteTag, ApplicantListRouteTag},
    scope::{
        applicant_display_name, apply_applicant_sort, find_applicant_scoped, scope_applicants,
    },
    state::JobPostingState,
    templates::applicants::{
        ApplicantCreateModalPage, ApplicantDeleteModalPage, ApplicantDetailPage,
        ApplicantEditModalPage, ApplicantFormValues, ApplicantListPage, ApplicantRow,
        HireApplicantModalPage,
    },
};
use lariv_plugin_hr::{logic::person::PersonInput, routes::EmployeeDetailRouteTag};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct ListQuery {
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub sort: Option<String>,
}

fn list_query_from_uri(uri: &Uri) -> ListQuery {
    let Some(query) = uri.query() else {
        return ListQuery::default();
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

pub(crate) fn applicant_input_from_form(
    form: &ApplicantForm,
    ctx: &AuthContext,
) -> Result<ApplicantInput, String> {
    Ok(ApplicantInput {
        person: PersonInput {
            name: form.name.clone(),
            mobile: form.mobile.clone(),
            email: form.email.clone(),
        },
        date_of_birth: parse_optional_datetime(&form.date_of_birth, &ctx.timezone)?,
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
    lariv_core::web::opt_or_log(JobFormEntity::find_by_id(id).one(db).await, "find job form")
        .map(|j| j.job_title)
        .unwrap_or_else(|| format!("Job posting #{id}"))
}

async fn resume_display(db: &sea_orm::DatabaseConnection, id: Option<i64>) -> String {
    let Some(id) = id.filter(|id| *id > 0) else {
        return String::new();
    };
    use lariv_plugin_filesystem::entities::filesystem_node::Entity as VNodeEntity;
    lariv_core::web::opt_or_log(
        VNodeEntity::find_by_id(id).one(db).await,
        "find resume vnode",
    )
    .map(|n| n.name)
    .unwrap_or_else(|| format!("File #{id}"))
}

async fn form_values_from_applicant(
    db: &sea_orm::DatabaseConnection,
    applicant: &applicant::Model,
    ctx: &AuthContext,
) -> ApplicantFormValues {
    ApplicantFormValues {
        name: applicant.name.clone(),
        mobile: applicant.mobile.clone(),
        email: applicant.email.clone(),
        date_of_birth: applicant
            .date_of_birth
            .map(|dt| ctx.datetime_local_input(dt).into_string())
            .unwrap_or_default(),
        gender: applicant
            .gender
            .map(|g| g.as_str().to_string())
            .unwrap_or_default(),
        address: applicant.address.clone().unwrap_or_default(),
        remarks: applicant.remarks.clone().unwrap_or_default(),
        job_form_id: fk_string(applicant.job_form_id),
        job_form_display: job_form_display(db, applicant.job_form_id).await,
        resume_vnode_id: fk_string(applicant.resume_vnode_id),
        resume_display: resume_display(db, applicant.resume_vnode_id).await,
    }
}

pub(crate) async fn form_values_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ApplicantForm,
) -> ApplicantFormValues {
    ApplicantFormValues {
        name: form.name.clone(),
        mobile: form.mobile.clone(),
        email: form.email.clone(),
        date_of_birth: form.date_of_birth.clone(),
        gender: form.gender.clone(),
        address: form.address.clone(),
        remarks: form.remarks.clone(),
        job_form_id: form.job_form_id.clone(),
        job_form_display: job_form_display(db, parse_optional_fk(&form.job_form_id)).await,
        resume_vnode_id: form.resume_vnode_id.clone(),
        resume_display: resume_display(db, parse_optional_fk(&form.resume_vnode_id)).await,
    }
}

pub(crate) async fn query_applicants(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    q: &ListQuery,
    page_size: u32,
    condition: Condition,
) -> (Vec<ApplicantRow>, u32, u64) {
    let page_num = q.page.get();
    let mut query = scope_applicants(ApplicantEntity::find(), auth);
    query = query.filter(condition);
    query = apply_applicant_sort(query, q.sort.as_deref());
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|a| ApplicantRow {
            id: a.id,
            name: applicant_display_name(&a),
            mobile: a.mobile,
            email: a.email,
            detail_href: ApplicantDetailRouteTag::new(a.id).url(),
        })
        .collect();
    (rows, page_num, total)
}

pub async fn list(
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    let q = list_query_from_uri(&uri);
    let prepared = crate::filters::prepare(
        uri.query().unwrap_or(""),
        crate::filters::applicant_fields(),
        state.db.get_database_backend(),
    );
    let (rows, page, total) =
        query_applicants(&state.db, &ctx, &q, q.page_size.get(), prepared.condition).await;
    let people = ObjectList::from_page(rows, page, q.page_size.get(), total);
    let page = ApplicantListPage {
        people,
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(APPLICANT_TABLE) {
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
    let page = ApplicantCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        values: ApplicantFormValues::default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ApplicantForm>,
) -> Response {
    match applicant_input_from_form(&form, &ctx) {
        Ok(input) => match create_applicant(&state.db, input).await {
            Ok(applicant) => respond_create_modal_done(
                &htmx,
                APPLICANT_CREATE_MODAL,
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
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(applicant) = find_applicant_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantListRouteTag.url()).into_response();
    };
    let values = form_values_from_applicant(&state.db, &applicant, &ctx).await;
    let page = ApplicantDetailPage {
        id: applicant.id,
        display_name: applicant_display_name(&applicant),
        values,
        job_form_href: applicant
            .job_form_id
            .filter(|id| *id > 0)
            .map(|id| crate::routes::JobFormDetailRouteTag::new(id).url())
            .unwrap_or_default(),
        answers: crate::questions::render_answers(&applicant.answers),
        resume_href: applicant
            .resume_vnode_id
            .filter(|id| *id > 0)
            .map(|id| lariv_plugin_filesystem::routes::VNodeDetailRouteTag::new(id).url())
            .unwrap_or_default(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(applicant) = find_applicant_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&ApplicantListRouteTag.url()).into_response();
    };
    let form_name = q.form_name();
    let page = ApplicantEditModalPage {
        id: applicant.id,
        form_name: form_name.clone(),
        post_url: modal_edit_post_url(ApplicantEditPostRouteTag::new(applicant.id), &form_name),
        values: form_values_from_applicant(&state.db, &applicant, &ctx).await,
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
    ctx: &lariv_plugin_users::state::AuthContext,
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
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ApplicantForm>,
) -> Response {
    let form_name = q.form_name();
    let post_url = modal_edit_post_url(ApplicantEditPostRouteTag::new(id), &form_name);
    match applicant_input_from_form(&form, &ctx) {
        Ok(input) => match update_applicant(&state.db, id, input).await {
            Ok(_) => respond_edit_modal_done(
                &htmx,
                APPLICANT_EDIT_MODAL,
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
    let _ = q;
    let page = ApplicantDeleteModalPage {
        id,
        message: "Are you sure you want to delete this applicant?".into(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match delete_applicant(&state.db, id).await {
        Ok(()) => htmx.redirect(&ApplicantListRouteTag.url()),
        Err(e) => {
            let page = ApplicantDeleteModalPage {
                id,
                message: "Are you sure you want to delete this applicant?".into(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn hire_get(
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_applicant_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&ApplicantListRouteTag.url()).into_response();
    }
    let page = HireApplicantModalPage {
        applicant_id: id,
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn hire_post(
    Cap(state): Cap<JobPostingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<HireApplicantBody>,
) -> Response {
    match hire_applicant(&state.db, id, &ctx).await {
        Ok(employee_id) => respond_create_modal_done(
            &htmx,
            HIRE_APPLICANT_MODAL,
            &q.refresh_table(),
            &EmployeeDetailRouteTag::new(employee_id).url(),
        ),
        Err(e) => {
            let page = HireApplicantModalPage {
                applicant_id: id,
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
