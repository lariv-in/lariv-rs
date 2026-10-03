use crate::plugins::users::role_authorization::scope_allowed;
use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{
    ColumnTrait, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait,
};

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx},
    html_form::{HtmlFormBody, UrlencodedFields},
    http::Cap,
    plugins::users::{entities::user, middleware::RequireAuth, state::AuthContext},
    web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::hr::{
    entities::attendance::{self, Entity as AttendanceEntity},
    forms::AttendanceForm,
    handlers::ModalNameQuery,
    keys::{AttendanceCreateModalKey, AttendanceEditModalKey, AttendanceTableKey},
    logic::attendance::{
        AttendanceInput, create_attendance, delete_attendance, update_attendance,
        user_display_label, user_display_labels,
    },
    routes::{AttendanceDetailRouteTag, AttendanceEditPostRouteTag, AttendanceListRouteTag},
    state::HrState,
    templates::attendances::{
        AttendanceCreateModalPage, AttendanceDeleteModalPage, AttendanceDetailPage,
        AttendanceEditModalPage, AttendanceListPage, AttendanceRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct AttendanceListQuery {
    #[serde(default, rename = "UserId", alias = "user_id")]
    pub user_id: Option<String>,
    #[serde(default, rename = "StartedAt", alias = "started_at")]
    pub started_at: Option<String>,
    #[serde(default, rename = "EndedAt", alias = "ended_at")]
    pub ended_at: Option<String>,
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

fn hub_query_from_uri(uri: &Uri) -> AttendanceListQuery {
    let Some(query) = uri.query() else {
        return AttendanceListQuery::default();
    };
    UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.deserialize().ok())
        .unwrap_or_default()
}

fn sort_desc(sort: &str) -> bool {
    sort.split_whitespace()
        .last()
        .is_some_and(|d| d.eq_ignore_ascii_case("DESC"))
}

fn parse_user_id(raw: Option<&str>) -> Option<i64> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    raw.parse::<i64>().ok().filter(|id| *id > 0)
}

fn fk_value(id: i64) -> String {
    if id <= 0 {
        String::new()
    } else {
        id.to_string()
    }
}

pub async fn list(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    let q = hub_query_from_uri(&uri);
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let mut query =
        scope_allowed::<super::super::routes::AttendanceView, _>(AttendanceEntity::find());
    if let Some(user_id) = parse_user_id(q.user_id.as_deref()) {
        query = query.filter(attendance::Column::UserId.eq(user_id));
    }
    if let Some(started_at) = q
        .started_at
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| ctx.parse_datetime_local_input(s))
    {
        query = query.filter(attendance::Column::StartedAt.gte(started_at));
    }
    if let Some(ended_at) = q
        .ended_at
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| ctx.parse_datetime_local_input(s))
    {
        query = query.filter(attendance::Column::EndedAt.lte(ended_at));
    }
    let sort = q.sort.as_deref().unwrap_or("");
    let desc = sort_desc(sort);
    let sort_key = sort.split_whitespace().next().unwrap_or("");
    if sort_key.eq_ignore_ascii_case("User") {
        query = query.join(JoinType::LeftJoin, attendance::Relation::User.def());
    }
    query = match sort_key {
        s if s.eq_ignore_ascii_case("User") => {
            if desc {
                query.order_by_desc(user::Column::Name)
            } else {
                query.order_by_asc(user::Column::Name)
            }
        }
        s if s.eq_ignore_ascii_case("StartedAt") => {
            if desc {
                query.order_by_desc(attendance::Column::StartedAt)
            } else {
                query.order_by_asc(attendance::Column::StartedAt)
            }
        }
        s if s.eq_ignore_ascii_case("EndedAt") => {
            if desc {
                query.order_by_desc(attendance::Column::EndedAt)
            } else {
                query.order_by_asc(attendance::Column::EndedAt)
            }
        }
        _ => query
            .order_by_desc(attendance::Column::StartedAt)
            .order_by_desc(attendance::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let names = user_display_labels(
        &state.db,
        &models.iter().map(|row| row.user_id).collect::<Vec<_>>(),
    )
    .await;
    let rows = models
        .into_iter()
        .map(|row| {
            let user = names
                .get(&row.user_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", row.user_id));
            AttendanceRow {
                id: row.id,
                user,
                started_at: ctx.format_datetime(row.started_at).into_string(),
                ended_at: ctx.format_datetime(row.ended_at).into_string(),
                detail_href: AttendanceDetailRouteTag::new(row.id).url(),
            }
        })
        .collect();
    let filter_user_id = parse_user_id(q.user_id.as_deref()).unwrap_or(0);
    let page = AttendanceListPage {
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        filter_user_id: fk_value(filter_user_id),
        filter_user_display: user_display_label(&state.db, filter_user_id).await,
        filter_started_at: q.started_at.unwrap_or_default(),
        filter_ended_at: q.ended_at.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
    };
    if htmx.targets::<AttendanceTableKey>() {
        return page.render_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let page = AttendanceCreateModalPage::new(q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<AttendanceForm>,
) -> Response {
    let user_display = user_display_label(&state.db, form.user_id).await;
    let input = match attendance_input_from_form(&ctx, &form) {
        Ok(input) => input,
        Err(e) => {
            let page = AttendanceCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                &form,
                user_display,
                e,
            );
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match create_attendance(&state.db, input).await {
        Ok(row) => respond_create_modal_done::<AttendanceCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &AttendanceDetailRouteTag::new(row.id).url(),
        ),
        Err(e) => {
            let page = AttendanceCreateModalPage::with_form(
                q.form_name(),
                q.refresh_table(),
                &form,
                user_display,
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
    let Some(row) = find_attendance_scoped(&state.db, id).await else {
        return Redirect::to(&AttendanceListRouteTag.url()).into_response();
    };
    let user = user_display_label(&state.db, row.user_id).await;
    let page = AttendanceDetailPage {
        id: row.id,
        user,
        started_at: ctx.format_datetime(row.started_at).into_string(),
        ended_at: ctx.format_datetime(row.ended_at).into_string(),
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
    let Some(row) = find_attendance_scoped(&state.db, id).await else {
        return Redirect::to(&AttendanceListRouteTag.url()).into_response();
    };
    let page = AttendanceEditModalPage {
        id: row.id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(AttendanceEditPostRouteTag::new(row.id), &q.form_name()),
        user_id: row.user_id,
        user_display: user_display_label(&state.db, row.user_id).await,
        started_at: ctx.datetime_local_input(row.started_at).into_string(),
        ended_at: ctx.datetime_local_input(row.ended_at).into_string(),
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
    HtmlFormBody(form): HtmlFormBody<AttendanceForm>,
) -> Response {
    let user_display = user_display_label(&state.db, form.user_id).await;
    let input = match attendance_input_from_form(&ctx, &form) {
        Ok(input) => input,
        Err(e) => {
            return edit_error_response(&chrome, &ctx, id, &q, &form, &user_display, e).await;
        }
    };
    match update_attendance(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<AttendanceEditModalKey>(
            &htmx,
            &AttendanceDetailRouteTag::new(id).url(),
        ),
        Err(e) => edit_error_response(&chrome, &ctx, id, &q, &form, &user_display, e).await,
    }
}

pub async fn delete_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_attendance_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&AttendanceListRouteTag.url()).into_response();
    }
    let page = AttendanceDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this attendance?".into(),
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
    match delete_attendance(&state.db, id).await {
        Ok(()) => htmx.redirect(&AttendanceListRouteTag.url()),
        Err(e) => {
            let page = AttendanceDeleteModalPage {
                id,
                form_name: q.form_name(),
                message: "Are you sure you want to delete this attendance?".into(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

fn attendance_input_from_form(
    ctx: &AuthContext,
    form: &AttendanceForm,
) -> Result<AttendanceInput, String> {
    if form.user_id <= 0 {
        return Err("user is required".to_string());
    }
    let started_raw = form.started_at.trim();
    if started_raw.is_empty() {
        return Err("start is required".to_string());
    }
    let ended_raw = form.ended_at.trim();
    if ended_raw.is_empty() {
        return Err("end is required".to_string());
    }
    let started_at = ctx
        .parse_datetime_local_input(started_raw)
        .ok_or_else(|| "invalid start".to_string())?;
    let ended_at = ctx
        .parse_datetime_local_input(ended_raw)
        .ok_or_else(|| "invalid end".to_string())?;
    Ok(AttendanceInput {
        user_id: form.user_id,
        started_at,
        ended_at,
    })
}

async fn edit_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &AttendanceForm,
    user_display: &str,
    error: String,
) -> Response {
    let page = AttendanceEditModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(AttendanceEditPostRouteTag::new(id), &q.form_name()),
        user_id: form.user_id,
        user_display: user_display.to_string(),
        started_at: form.started_at.clone(),
        ended_at: form.ended_at.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn find_attendance_scoped(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Option<attendance::Model> {
    crate::web::opt_or_log(
        scope_allowed::<super::super::routes::AttendanceView, _>(AttendanceEntity::find_by_id(id))
            .one(db)
            .await,
        "find attendance by id",
    )
}
