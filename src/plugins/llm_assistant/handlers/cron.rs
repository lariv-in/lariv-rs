use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};
use serde::Deserialize;

use crate::template::RenderAppPane;
use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    duration::format_duration,
    html_form::HtmlFormBody,
    http::Cap,
    plugins::{
        llm_assistant::{
            cron::parse_job_duration,
            entities::{
                cron_job::{self, Entity as CronJobEntity},
                cron_job_run::{self, Entity as CronJobRunEntity},
                session::{self, Entity as SessionEntity},
            },
            forms::CronJobForm,
            handlers::history::session_display_title,
            keys::{
                CronJobCreateModalKey, CronJobDeleteModalKey, CronJobEditModalKey, CronJobsTableKey,
            },
            routes::CronJobsDetailRouteTag,
            state::LlmAssistantState,
            templates::{
                CronJobConfirmDeletePage, CronJobCreateModalPage, CronJobDetailPage,
                CronJobEditModalPage, CronJobListPage, CronJobRow, CronJobRunRow,
            },
        },
        users::middleware::RequireAuth,
    },
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use super::ModalNameQuery;

const RECENT_RUNS: u64 = 50;

#[derive(Debug, Deserialize, Default)]
pub struct CronJobListQuery {
    #[serde(default, rename = "Prompt", alias = "prompt")]
    pub prompt: Option<String>,
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

fn format_last_activation(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    match dt {
        Some(d) => crate::datetime::DatetimeLabel::short(d, tz).into_string(),
        None => "Never".to_string(),
    }
}

async fn latest_runs_for_jobs(
    db: &sea_orm::DatabaseConnection,
    job_ids: &[i64],
) -> std::collections::HashMap<i64, chrono::DateTime<Utc>> {
    let mut map = std::collections::HashMap::new();
    if job_ids.is_empty() {
        return map;
    }
    let runs = CronJobRunEntity::find()
        .filter(cron_job_run::Column::CronJobId.is_in(job_ids.to_vec()))
        .order_by_desc(cron_job_run::Column::Datetime)
        .all(db)
        .await
        .unwrap_or_default();
    for run in runs {
        map.entry(run.cron_job_id).or_insert(run.datetime);
    }
    map
}

async fn query_jobs(
    db: &sea_orm::DatabaseConnection,
    q: &CronJobListQuery,
) -> (Vec<cron_job::Model>, u32, u64) {
    let mut query = CronJobEntity::find();
    let prompt = q.prompt.clone().unwrap_or_default();
    if !prompt.is_empty() {
        query = crate::db::trigram::apply_text_search(
            query,
            db.get_database_backend(),
            &[cron_job::Column::Prompt],
            &prompt,
        );
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Duration DESC") => {
            query.order_by_desc(cron_job::Column::Duration)
        }
        s if s.eq_ignore_ascii_case("Duration ASC") || s.eq_ignore_ascii_case("Duration") => {
            query.order_by_asc(cron_job::Column::Duration)
        }
        s if s.eq_ignore_ascii_case("Prompt DESC") => query.order_by_desc(cron_job::Column::Prompt),
        s if s.eq_ignore_ascii_case("Prompt ASC") || s.eq_ignore_ascii_case("Prompt") => {
            query.order_by_asc(cron_job::Column::Prompt)
        }
        _ => query.order_by_desc(cron_job::Column::Id),
    };

    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn load_jobs_page(
    db: &sea_orm::DatabaseConnection,
    q: &CronJobListQuery,
    tz: &str,
) -> ObjectList<CronJobRow> {
    let (models, page, total) = query_jobs(db, q).await;
    let ids: Vec<i64> = models.iter().map(|j| j.id).collect();
    let last_runs = latest_runs_for_jobs(db, &ids).await;
    let rows = models
        .into_iter()
        .map(|j| CronJobRow {
            id: j.id,
            duration: format_duration(j.duration),
            prompt: crate::plugins::llm_assistant::handlers::history::title_from_first_prompt(
                &j.prompt,
            ),
            last_activation: format_last_activation(last_runs.get(&j.id).copied(), tz),
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

async fn load_recent_runs(
    db: &sea_orm::DatabaseConnection,
    job_id: i64,
    tz: &str,
) -> Vec<CronJobRunRow> {
    let runs = CronJobRunEntity::find()
        .filter(cron_job_run::Column::CronJobId.eq(job_id))
        .order_by_desc(cron_job_run::Column::Datetime)
        .paginate(db, RECENT_RUNS)
        .fetch_page(0)
        .await
        .unwrap_or_default();
    let session_ids: Vec<i64> = runs.iter().filter_map(|r| r.session_id).collect();
    let sessions = if session_ids.is_empty() {
        Vec::new()
    } else {
        SessionEntity::find()
            .filter(session::Column::Id.is_in(session_ids))
            .all(db)
            .await
            .unwrap_or_default()
    };
    runs.into_iter()
        .map(|r| {
            let conversation = r.session_id.and_then(|sid| {
                sessions
                    .iter()
                    .find(|s| s.id == sid)
                    .map(|s| session_display_title(s.id, &s.title))
            });
            CronJobRunRow {
                datetime: crate::datetime::DatetimeLabel::short(r.datetime, tz).into_string(),
                session_id: r.session_id,
                conversation: conversation.unwrap_or_else(|| {
                    r.session_id
                        .map(|id| format!("Session #{id}"))
                        .unwrap_or_else(|| "—".to_string())
                }),
            }
        })
        .collect()
}

fn parsed_form(form: &CronJobForm) -> Result<(i64, String), String> {
    let prompt = form.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("prompt is required".to_string());
    }
    let duration = parse_job_duration(&form.duration)?;
    Ok((duration, prompt))
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CronJobListQuery>,
) -> maud::Markup {
    let jobs = load_jobs_page(&state.db, &q, &ctx.timezone).await;
    let page = CronJobListPage {
        jobs,
        filter_prompt: q.prompt.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    if htmx.targets::<CronJobsTableKey>() {
        return page.render_table();
    }
    if htmx.wants_main_content() {
        return page.render_main().into();
    }
    if htmx.wants_app_layout() {
        return page.render_pane().into();
    }
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

async fn load_detail_page(
    db: &sea_orm::DatabaseConnection,
    job: cron_job::Model,
    tz: &str,
    error: String,
) -> CronJobDetailPage {
    let last = CronJobRunEntity::find()
        .filter(cron_job_run::Column::CronJobId.eq(job.id))
        .order_by_desc(cron_job_run::Column::Datetime)
        .one(db)
        .await
        .ok()
        .flatten();
    let runs = load_recent_runs(db, job.id, tz).await;
    CronJobDetailPage {
        id: job.id,
        duration: format_duration(job.duration),
        prompt: job.prompt,
        last_activation: format_last_activation(last.map(|r| r.datetime), tz),
        runs,
        error,
    }
}

/// HTTP handler: `detail`.
pub async fn detail(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(job) = crate::web::opt_or_log(
        CronJobEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/cron-jobs/").into_response();
    };
    let page = load_detail_page(&state.db, job, &ctx.timezone, String::new()).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `run_post`.
pub async fn run_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(job) = crate::web::opt_or_log(
        CronJobEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/cron-jobs/").into_response();
    };
    match state.cron_scheduler.run_now(&state, job.clone()).await {
        Ok(()) => htmx.redirect(&CronJobsDetailRouteTag::new(id).url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to run cron job");
            let page = load_detail_page(&state.db, job, &ctx.timezone, e.to_string()).await;
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

/// HTTP handler: `create_get`.
pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = CronJobCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        duration: String::new(),
        prompt: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `create_post`.
pub async fn create_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CronJobForm>,
) -> Response {
    let create_err = |error: String, form: CronJobForm| {
        let page = CronJobCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            duration: form.duration,
            prompt: form.prompt,
            error,
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };

    let (duration, prompt) = match parsed_form(&form) {
        Ok(v) => v,
        Err(e) => return create_err(e, form),
    };
    let now = Utc::now();
    let model = cron_job::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        duration: Set(duration),
        prompt: Set(prompt),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            state.cron_scheduler.reload();
            respond_create_modal_done::<CronJobCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &CronJobsDetailRouteTag::new(saved.id).url(),
            )
        }
        Err(e) => create_err(e.to_string(), form),
    }
}

/// HTTP handler: `edit_get`.
pub async fn edit_get(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(job) = crate::web::opt_or_log(
        CronJobEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/cron-jobs/").into_response();
    };
    let page = CronJobEditModalPage {
        id: job.id,
        form_name: q.form_name(),
        duration: format_duration(job.duration),
        prompt: job.prompt,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

/// HTTP handler: `edit_post`.
pub async fn edit_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CronJobForm>,
) -> Response {
    let Some(job) = crate::web::opt_or_log(
        CronJobEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    ) else {
        return Redirect::to("/llm-assistant/cron-jobs/").into_response();
    };

    let edit_err = |error: String, form: CronJobForm| {
        let page = CronJobEditModalPage {
            id,
            form_name: q.form_name(),
            duration: form.duration,
            prompt: form.prompt,
            error,
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };

    let (duration, prompt) = match parsed_form(&form) {
        Ok(v) => v,
        Err(e) => return edit_err(e, form),
    };
    let mut am: cron_job::ActiveModel = job.into();
    am.duration = Set(duration);
    am.prompt = Set(prompt);
    am.updated_at = Set(Some(Utc::now()));
    match am.update(&state.db).await {
        Ok(_) => {
            state.cron_scheduler.reload();
            respond_edit_modal_done::<CronJobEditModalKey>(
                &htmx,
                &CronJobsDetailRouteTag::new(id).url(),
            )
        }
        Err(e) => edit_err(e.to_string(), form),
    }
}

/// HTTP handler: `delete_get`.
pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = CronJobConfirmDeletePage {
        modal_uid: CronJobDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this cron job?".into(),
        name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_llm_assistant.CronJobDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

/// HTTP handler: `delete_post`.
pub async fn delete_post(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match CronJobEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => {
            state.cron_scheduler.reload();
            htmx.redirect("/llm-assistant/cron-jobs/")
        }
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete cron job");
            let page = CronJobConfirmDeletePage {
                modal_uid: CronJobDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this cron job?".into(),
                name: "p_llm_assistant.CronJobDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}
