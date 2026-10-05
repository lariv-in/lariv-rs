use lariv_plugin_users::role_authorization::scope_allowed;
use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    sea_query::{Query as SeaQuery, SelectStatement},
};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::datetime::parse_date;
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_plugin_users::{middleware::RequireAuth, roles::Superuser, state::AuthContext};
use lariv_core::web::{
        Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
    };

use crate::{
    entities::leaves::{
        approved_leave, leave_application, leave_application::Entity as LeaveApplicationEntity,
        leave_type::LeaveType, rejected_leave,
    },
    forms::{
        ApproveLeaveForm, LeaveApplicationForm, RejectLeaveForm, RevokeApprovalForm,
        RevokeRejectionForm,
    },
    handlers::ModalNameQuery,
    keys::{
        LeaveApproveModalKey, LeaveCreateModalKey, LeaveEditModalKey, LeaveRejectModalKey,
        LeaveRevokeApprovalModalKey, LeaveRevokeRejectionModalKey, LeaveTableKey,
    },
    logic::{
        attendance::{user_display_label, user_display_labels},
        leave::{
            ApproveLeaveInput, FILTER_APPROVED, FILTER_PENDING, FILTER_REJECTED,
            LeaveApplicationInput, RejectLeaveInput, approve_leave, approved_application_ids,
            create_leave_application, delete_leave_application, ensure_leave_approver,
            ensure_leave_editor, ensure_leave_rejector, find_approval, find_rejection,
            reject_leave, rejected_application_ids, revoke_approval, revoke_rejection,
            status_label, update_leave_application,
        },
    },
    routes::{
        LeaveApprovePostRouteTag, LeaveDetailRouteTag, LeaveEditPostRouteTag, LeaveListRouteTag,
        LeaveRejectPostRouteTag, LeaveRevokeApprovalPostRouteTag, LeaveRevokeRejectionPostRouteTag,
    },
    state::HrState,
    templates::leaves::{
        LeaveApproveModalPage, LeaveCreateModalPage, LeaveDeleteModalPage, LeaveDetailPage,
        LeaveEditModalPage, LeaveListPage, LeaveRejectModalPage, LeaveRevokeApprovalModalPage,
        LeaveRevokeRejectionModalPage, LeaveRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct LeaveListQuery {
    #[serde(
        default,
        rename = "AppliedByID",
        alias = "AppliedById",
        alias = "applied_by_id"
    )]
    pub applied_by_id: Option<String>,
    #[serde(default, rename = "Date", alias = "date")]
    pub date: Option<String>,
    #[serde(default, rename = "LeaveType", alias = "leave_type")]
    pub leave_type: Option<String>,
    #[serde(default, rename = "Status", alias = "status")]
    pub status: Option<String>,
    #[serde(default, rename = "Reason", alias = "reason")]
    pub reason: Option<String>,
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

fn hub_query_from_uri(uri: &Uri) -> LeaveListQuery {
    let Some(query) = uri.query() else {
        return LeaveListQuery::default();
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

fn approved_id_subquery() -> SelectStatement {
    SeaQuery::select()
        .column(approved_leave::Column::LeaveApplicationId)
        .from(approved_leave::Entity)
        .to_owned()
}

fn rejected_id_subquery() -> SelectStatement {
    SeaQuery::select()
        .column(rejected_leave::Column::LeaveApplicationId)
        .from(rejected_leave::Entity)
        .to_owned()
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
        scope_allowed::<super::super::routes::LeaveView, _>(LeaveApplicationEntity::find());
    if let Some(id) = parse_user_id(q.applied_by_id.as_deref()) {
        query = query.filter(leave_application::Column::AppliedById.eq(id));
    }
    if let Some(date) = q
        .date
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(parse_date)
    {
        query = query.filter(leave_application::Column::Date.eq(date));
    }
    if let Some(kind) = q
        .leave_type
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(LeaveType::parse)
    {
        query = query.filter(leave_application::Column::LeaveType.eq(kind));
    }
    if let Some(reason) = q.reason.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(leave_application::Column::Reason.contains(reason));
    }
    query = match q.status.as_deref().unwrap_or("").trim() {
        FILTER_APPROVED => {
            query.filter(leave_application::Column::Id.in_subquery(approved_id_subquery()))
        }
        FILTER_REJECTED => query
            .filter(leave_application::Column::Id.in_subquery(rejected_id_subquery()))
            .filter(leave_application::Column::Id.not_in_subquery(approved_id_subquery())),
        FILTER_PENDING => query
            .filter(leave_application::Column::Id.not_in_subquery(approved_id_subquery()))
            .filter(leave_application::Column::Id.not_in_subquery(rejected_id_subquery())),
        _ => query,
    };
    let sort = q.sort.as_deref().unwrap_or("");
    let desc = sort_desc(sort);
    query = match sort.split_whitespace().next().unwrap_or("") {
        s if s.eq_ignore_ascii_case("AppliedBy") => {
            if desc {
                query.order_by_desc(leave_application::Column::AppliedById)
            } else {
                query.order_by_asc(leave_application::Column::AppliedById)
            }
        }
        s if s.eq_ignore_ascii_case("LeaveType") => {
            if desc {
                query.order_by_desc(leave_application::Column::LeaveType)
            } else {
                query.order_by_asc(leave_application::Column::LeaveType)
            }
        }
        s if s.eq_ignore_ascii_case("Date") => {
            if desc {
                query.order_by_desc(leave_application::Column::Date)
            } else {
                query.order_by_asc(leave_application::Column::Date)
            }
        }
        _ => query
            .order_by_desc(leave_application::Column::Date)
            .order_by_desc(leave_application::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = models.iter().map(|row| row.id).collect();
    let user_ids: Vec<i64> = models.iter().map(|row| row.applied_by_id).collect();
    let names = user_display_labels(&state.db, &user_ids).await;
    let approved = approved_application_ids(&state.db, &ids).await;
    let rejected = rejected_application_ids(&state.db, &ids).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let applied_by = names
                .get(&row.applied_by_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", row.applied_by_id));
            LeaveRow {
                id: row.id,
                applied_by,
                date: lariv_core::datetime::format_date(row.date),
                leave_type: row.leave_type.label().to_string(),
                status: status_label(approved.contains(&row.id), rejected.contains(&row.id))
                    .to_string(),
                reason: row.reason,
                detail_href: LeaveDetailRouteTag::new(row.id).url(),
            }
        })
        .collect();
    let filter_applied_by_id = parse_user_id(q.applied_by_id.as_deref()).unwrap_or(0);
    let page = LeaveListPage {
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        filter_applied_by_id: fk_value(filter_applied_by_id),
        filter_applied_by_display: user_display_label(&state.db, filter_applied_by_id).await,
        filter_date: q.date.unwrap_or_default(),
        filter_leave_type: q.leave_type.unwrap_or_default(),
        filter_status: q.status.unwrap_or_default(),
        filter_reason: q.reason.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
    };
    if htmx.targets::<LeaveTableKey>() {
        return page.render_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let page = LeaveCreateModalPage::new(q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<LeaveApplicationForm>,
) -> Response {
    let input = match application_input_from_form(ctx.user.id, &form) {
        Ok(input) => input,
        Err(e) => {
            let page = LeaveCreateModalPage::with_form(q.form_name(), q.refresh_table(), &form, e);
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match create_leave_application(&state.db, input).await {
        Ok(row) => respond_create_modal_done::<LeaveCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &LeaveDetailRouteTag::new(row.id).url(),
        ),
        Err(e) => {
            let page = LeaveCreateModalPage::with_form(q.form_name(), q.refresh_table(), &form, e);
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
    let Some(row) = find_leave_scoped(&state.db, id).await else {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    };
    let page = detail_page(&state.db, &ctx, row).await;
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_leave_scoped(&state.db, id).await else {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    };
    if ensure_leave_editor(&ctx, row.applied_by_id).is_err() {
        return Redirect::to(&LeaveDetailRouteTag::new(id).url()).into_response();
    }
    let page = LeaveEditModalPage {
        id: row.id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveEditPostRouteTag::new(row.id), &q.form_name()),
        date: lariv_core::datetime::format_date(row.date),
        leave_type: row.leave_type.as_str().to_string(),
        reason: row.reason,
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
    HtmlFormBody(form): HtmlFormBody<LeaveApplicationForm>,
) -> Response {
    let Some(existing) = find_leave_scoped(&state.db, id).await else {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    };
    if let Err(e) = ensure_leave_editor(&ctx, existing.applied_by_id) {
        return edit_error_response(&chrome, &ctx, id, &q, &form, e).await;
    }
    let input = match application_input_from_form(existing.applied_by_id, &form) {
        Ok(input) => input,
        Err(e) => {
            return edit_error_response(&chrome, &ctx, id, &q, &form, e).await;
        }
    };
    match update_leave_application(&state.db, id, input).await {
        Ok(_) => {
            respond_edit_modal_done::<LeaveEditModalKey>(&htmx, &LeaveDetailRouteTag::new(id).url())
        }
        Err(e) => edit_error_response(&chrome, &ctx, id, &q, &form, e).await,
    }
}

pub async fn delete_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_leave_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    }
    let page = LeaveDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this leave application?".into(),
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
    match delete_leave_application(&state.db, id).await {
        Ok(()) => htmx.redirect(&LeaveListRouteTag.url()),
        Err(e) => {
            let page = LeaveDeleteModalPage {
                id,
                form_name: q.form_name(),
                message: "Are you sure you want to delete this leave application?".into(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn approve_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(leave) = find_leave_scoped(&state.db, id).await else {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    };
    if !is_pending(&state.db, id).await {
        return Redirect::to(&LeaveDetailRouteTag::new(id).url()).into_response();
    }
    if ensure_leave_approver(&state.db, &ctx, leave.applied_by_id)
        .await
        .is_err()
    {
        return Redirect::to(&LeaveDetailRouteTag::new(id).url()).into_response();
    }
    let page = LeaveApproveModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveApprovePostRouteTag::new(id), &q.form_name()),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn approve_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<ApproveLeaveForm>,
) -> Response {
    let Some(leave) = find_leave_scoped(&state.db, id).await else {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    };
    if let Err(e) = ensure_leave_approver(&state.db, &ctx, leave.applied_by_id).await {
        return approve_error_response(&chrome, &ctx, id, &q, e).await;
    }
    let input = ApproveLeaveInput {
        approved_by_id: ctx.user.id,
        approved_at: Utc::now(),
    };
    match approve_leave(&state.db, id, &ctx, input).await {
        Ok(_) => respond_edit_modal_done::<LeaveApproveModalKey>(
            &htmx,
            &LeaveDetailRouteTag::new(id).url(),
        ),
        Err(e) => approve_error_response(&chrome, &ctx, id, &q, e).await,
    }
}

pub async fn reject_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_leave_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    }
    if !is_pending(&state.db, id).await {
        return Redirect::to(&LeaveDetailRouteTag::new(id).url()).into_response();
    }
    let page = LeaveRejectModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveRejectPostRouteTag::new(id), &q.form_name()),
        reason: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn reject_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<RejectLeaveForm>,
) -> Response {
    let input = RejectLeaveInput {
        rejected_by_id: ctx.user.id,
        rejected_at: Utc::now(),
        reason: optional_reason(&form.reason),
    };
    match reject_leave(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<LeaveRejectModalKey>(
            &htmx,
            &LeaveDetailRouteTag::new(id).url(),
        ),
        Err(e) => reject_error_response(&chrome, &ctx, id, &q, &form, e).await,
    }
}

pub async fn revoke_approval_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(leave) = find_leave_scoped(&state.db, id).await else {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    };
    if find_approval(&state.db, id).await.is_none()
        || ensure_leave_approver(&state.db, &ctx, leave.applied_by_id)
            .await
            .is_err()
    {
        return Redirect::to(&LeaveDetailRouteTag::new(id).url()).into_response();
    }
    let page = LeaveRevokeApprovalModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveRevokeApprovalPostRouteTag::new(id), &q.form_name()),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn revoke_approval_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<RevokeApprovalForm>,
) -> Response {
    match revoke_approval(&state.db, id, &ctx).await {
        Ok(()) => respond_edit_modal_done::<LeaveRevokeApprovalModalKey>(
            &htmx,
            &LeaveDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = LeaveRevokeApprovalModalPage {
                id,
                form_name: q.form_name(),
                post_url: modal_edit_post_url(
                    LeaveRevokeApprovalPostRouteTag::new(id),
                    &q.form_name(),
                ),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn revoke_rejection_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if find_leave_scoped(&state.db, id).await.is_none() || ensure_leave_rejector(&ctx).is_err() {
        return Redirect::to(&LeaveListRouteTag.url()).into_response();
    }
    if find_rejection(&state.db, id).await.is_none() {
        return Redirect::to(&LeaveDetailRouteTag::new(id).url()).into_response();
    }
    let page = LeaveRevokeRejectionModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveRevokeRejectionPostRouteTag::new(id), &q.form_name()),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn revoke_rejection_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(_form): HtmlFormBody<RevokeRejectionForm>,
) -> Response {
    match revoke_rejection(&state.db, id, &ctx).await {
        Ok(()) => respond_edit_modal_done::<LeaveRevokeRejectionModalKey>(
            &htmx,
            &LeaveDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = LeaveRevokeRejectionModalPage {
                id,
                form_name: q.form_name(),
                post_url: modal_edit_post_url(
                    LeaveRevokeRejectionPostRouteTag::new(id),
                    &q.form_name(),
                ),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

async fn detail_page(
    db: &sea_orm::DatabaseConnection,
    ctx: &AuthContext,
    row: leave_application::Model,
) -> LeaveDetailPage {
    let applied_by = user_display_label(db, row.applied_by_id).await;
    let can_approve = ensure_leave_approver(db, ctx, row.applied_by_id)
        .await
        .is_ok();
    let can_edit = ensure_leave_editor(ctx, row.applied_by_id).is_ok();
    let approved = find_approval(db, row.id).await;
    let rejected = find_rejection(db, row.id).await;
    let can_revoke_approval = approved.is_some() && can_approve;
    let can_revoke_rejection = rejected.is_some() && Superuser::matches(&ctx.role);
    let approved_by = match &approved {
        Some(decision) => user_display_label(db, decision.approved_by_id).await,
        None => String::new(),
    };
    let rejected_by = match &rejected {
        Some(decision) => user_display_label(db, decision.rejected_by_id).await,
        None => String::new(),
    };
    LeaveDetailPage {
        id: row.id,
        title: applied_by.clone(),
        applied_by,
        date: lariv_core::datetime::format_date(row.date),
        leave_type: row.leave_type.label().to_string(),
        reason: row.reason,
        status: status_label(approved.is_some(), rejected.is_some()).to_string(),
        approved_by,
        approved_at: approved
            .map(|decision| ctx.format_datetime(decision.approved_at).into_string())
            .unwrap_or_default(),
        rejected_by,
        rejected_at: rejected
            .as_ref()
            .map(|decision| ctx.format_datetime(decision.rejected_at).into_string())
            .unwrap_or_default(),
        rejection_reason: rejected
            .and_then(|decision| decision.reason)
            .unwrap_or_default(),
        can_approve,
        can_edit,
        can_revoke_approval,
        can_revoke_rejection,
    }
}

fn application_input_from_form(
    applied_by_id: i64,
    form: &LeaveApplicationForm,
) -> Result<LeaveApplicationInput, String> {
    if applied_by_id <= 0 {
        return Err("applied by is required".to_string());
    }
    let date_raw = form.date.trim();
    if date_raw.is_empty() {
        return Err("date is required".to_string());
    }
    let date = parse_date(date_raw).ok_or_else(|| "invalid date".to_string())?;
    let leave_type = LeaveType::parse(form.leave_type.trim())
        .ok_or_else(|| "leave type is required".to_string())?;
    Ok(LeaveApplicationInput {
        applied_by_id,
        date,
        reason: form.reason.clone(),
        leave_type,
    })
}

fn optional_reason(reason: &str) -> Option<String> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn edit_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &LeaveApplicationForm,
    error: String,
) -> Response {
    let page = LeaveEditModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveEditPostRouteTag::new(id), &q.form_name()),
        date: form.date.clone(),
        leave_type: form.leave_type.clone(),
        reason: form.reason.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn approve_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    error: String,
) -> Response {
    let page = LeaveApproveModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveApprovePostRouteTag::new(id), &q.form_name()),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn reject_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &RejectLeaveForm,
    error: String,
) -> Response {
    let page = LeaveRejectModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(LeaveRejectPostRouteTag::new(id), &q.form_name()),
        reason: form.reason.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn is_pending(db: &sea_orm::DatabaseConnection, id: i64) -> bool {
    find_approval(db, id).await.is_none() && find_rejection(db, id).await.is_none()
}

async fn find_leave_scoped(
    db: &sea_orm::DatabaseConnection,
    id: i64,
) -> Option<leave_application::Model> {
    lariv_core::web::opt_or_log(
        scope_allowed::<super::super::routes::LeaveView, _>(LeaveApplicationEntity::find_by_id(id))
            .one(db)
            .await,
        "find leave application by id",
    )
}

#[cfg(test)]
mod tests {
    use super::LeaveListQuery;
    use lariv_core::html_form::UrlencodedFields;

    #[test]
    fn filter_query_accepts_the_form_field_names() {
        let q: LeaveListQuery = UrlencodedFields::parse(
            b"AppliedByID=4&Date=2026-10-04&LeaveType=privilege&Status=pending&Reason=fever",
        )
        .unwrap()
        .deserialize()
        .unwrap();
        assert_eq!(q.applied_by_id.as_deref(), Some("4"));
        assert_eq!(q.date.as_deref(), Some("2026-10-04"));
        assert_eq!(q.leave_type.as_deref(), Some("privilege"));
        assert_eq!(q.status.as_deref(), Some("pending"));
        assert_eq!(q.reason.as_deref(), Some("fever"));
    }
}
