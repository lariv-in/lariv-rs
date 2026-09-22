use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx},
    html_form::{HtmlFormBody, UrlencodedFields},
    http::Cap,
    plugins::{forms::handlers::forms::find_form, users::middleware::RequireAuth},
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::hr::{
    entities::job_form::{self, Entity as JobFormEntity},
    forms::JobFormForm,
    handlers::ModalNameQuery,
    keys::{JobFormCreateModalKey, JobFormEditModalKey},
    logic::job_form::{JobFormInput, create_job_form, delete_job_form, update_job_form},
    routes::{
        JobApplicationPublicGetRouteTag, JobFormDetailRouteTag, JobFormEditPostRouteTag,
        JobFormListRouteTag,
    },
    scope::scope_job_forms,
    state::HrState,
    templates::job_forms::{
        JobFormCreateModalPage, JobFormDeleteModalPage, JobFormDetailPage, JobFormEditModalPage,
        JobFormListPage, JobFormRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct JobFormListQuery {
    #[serde(default, rename = "Title", alias = "title")]
    pub title: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

async fn form_title(db: &sea_orm::DatabaseConnection, form_id: i64) -> String {
    find_form(db, form_id)
        .await
        .map(|f| f.title)
        .unwrap_or_else(|| format!("Form #{form_id}"))
}

pub async fn list(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let q = hub_query_from_uri(&uri);
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let mut query = scope_job_forms(JobFormEntity::find(), &ctx);
    if let Some(title) = q.title.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(job_form::Column::JobTitle.contains(title));
    }
    let sort = q.sort.as_deref().unwrap_or("");
    query = match sort.split_whitespace().next().unwrap_or("") {
        s if s.eq_ignore_ascii_case("Title") => {
            if sort
                .split_whitespace()
                .last()
                .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
            {
                query.order_by_desc(job_form::Column::JobTitle)
            } else {
                query.order_by_asc(job_form::Column::JobTitle)
            }
        }
        _ => query.order_by_desc(job_form::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::new();
    for job in models {
        rows.push(JobFormRow {
            id: job.id,
            job_title: job.job_title,
            form_title: form_title(&state.db, job.form_id).await,
            apply_href: JobApplicationPublicGetRouteTag::new(job.id).url(),
            detail_href: JobFormDetailRouteTag::new(job.id).url(),
        });
    }
    let page = JobFormListPage {
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        filter_title: q.title.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: true,
        page_size,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn hub_query_from_uri(uri: &Uri) -> JobFormListQuery {
    let Some(query) = uri.query() else {
        return JobFormListQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let page = JobFormCreateModalPage::new(q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<JobFormForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let input = job_form_input_from_form(&form);
    match create_job_form(&state.db, input).await {
        Ok(job) => respond_create_modal_done::<JobFormCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &JobFormDetailRouteTag::new(job.id).url(),
        ),
        Err(e) => {
            let page =
                JobFormCreateModalPage::with_form(q.form_name(), q.refresh_table(), &form, e);
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
    let Some(job) = scope_job_forms(JobFormEntity::find_by_id(id), &ctx)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to("/hr/job-forms").into_response();
    };
    let page = JobFormDetailPage {
        id: job.id,
        job_title: job.job_title,
        salary_range: job.salary_range.unwrap_or_default(),
        experience_required: job.experience_required.unwrap_or_default(),
        description: job.description,
        form_title: form_title(&state.db, job.form_id).await,
        apply_href: JobApplicationPublicGetRouteTag::new(job.id).url(),
        can_edit: ctx.user.is_superuser,
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
    let Some(job) = find_job_form_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/hr/job-forms").into_response();
    };
    let page = JobFormEditModalPage {
        id: job.id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(JobFormEditPostRouteTag::new(job.id), &q.form_name()),
        job_title: job.job_title,
        salary_range: job.salary_range.unwrap_or_default(),
        experience_required: job.experience_required.unwrap_or_default(),
        description: job.description,
        form_id: job.form_id,
        form_display: form_title(&state.db, job.form_id).await,
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
    HtmlFormBody(form): HtmlFormBody<JobFormForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    let input = job_form_input_from_form(&form);
    match update_job_form(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<JobFormEditModalKey>(
            &htmx,
            &JobFormDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = JobFormEditModalPage {
                id,
                form_name: q.form_name(),
                post_url: modal_edit_post_url(JobFormEditPostRouteTag::new(id), &q.form_name()),
                job_title: form.job_title.clone(),
                salary_range: form.salary_range.clone(),
                experience_required: form.experience_required.clone(),
                description: form.description.clone(),
                form_id: form.form_id,
                form_display: form_title(&state.db, form.form_id).await,
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
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    if find_job_form_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to("/hr/job-forms").into_response();
    }
    let page = JobFormDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this job posting?".into(),
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
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/hr/applicants").into_response();
    }
    match delete_job_form(&state.db, id).await {
        Ok(()) => htmx.redirect(&JobFormListRouteTag.url()),
        Err(e) => {
            let page = JobFormDeleteModalPage {
                id,
                form_name: q.form_name(),
                message: "Are you sure you want to delete this job posting?".into(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

fn job_form_input_from_form(form: &JobFormForm) -> JobFormInput {
    JobFormInput {
        job_title: form.job_title.clone(),
        salary_range: Some(form.salary_range.clone()),
        experience_required: Some(form.experience_required.clone()),
        description: form.description.clone(),
        form_id: form.form_id,
    }
}

async fn find_job_form_scoped(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    ctx: &crate::plugins::users::state::AuthContext,
) -> Option<job_form::Model> {
    crate::web::opt_or_log(
        scope_job_forms(JobFormEntity::find_by_id(id), ctx)
            .one(db)
            .await,
        "find job form by id",
    )
}
