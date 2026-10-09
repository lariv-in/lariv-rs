use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{DateTime, Utc};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    sea_query::{Expr, Query as SeaQuery, SelectStatement},
};

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::web::{
    Htmx, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    modal_edit_post_url, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::{middleware::RequireAuth, roles::Superuser, state::AuthContext};

use crate::{
    entities::overtime::{
        approved_overtime::{self, Entity as ApprovedOvertimeEntity},
        overtime_application::{self, Entity as OvertimeApplicationEntity},
        rejected_overtime,
    },
    forms::{
        ApproveOvertimeForm, ApprovedOvertimeForm, OvertimeApplicationForm, RejectOvertimeForm,
        RevokeOvertimeApprovalForm, RevokeOvertimeRejectionForm,
    },
    handlers::ModalNameQuery,
    keys::{
        ApprovedOvertimeCreateModalKey, ApprovedOvertimeTableKey, OvertimeApproveModalKey,
        OvertimeCreateModalKey, OvertimeEditModalKey, OvertimeRejectModalKey,
        OvertimeRevokeApprovalModalKey, OvertimeRevokeRejectionModalKey, OvertimeTableKey,
    },
    logic::{
        attendance::{user_display_label, user_display_labels},
        overtime::{
            ApproveOvertimeInput, ApprovedOvertimeInput, FILTER_APPROVED, FILTER_PENDING,
            FILTER_REJECTED, OvertimeApplicationInput, RejectOvertimeInput, STATUS_APPROVED,
            STATUS_REJECTED, actor_may_view_leave, applicant_manager_id, approve_overtime,
            approved_application_ids, create_approved_overtime, create_overtime_application,
            delete_overtime_application, ensure_overtime_approver, ensure_overtime_editor,
            find_approval, find_rejection, managed_applicant_ids, optional_reason, reject_overtime,
            rejected_application_ids, revoke_approval, revoke_rejection, status_label,
            update_overtime_application, user_is_leave_manager,
        },
    },
    routes::{
        OvertimeApplicationsRouteTag, OvertimeApprovedRouteTag, OvertimeApprovalsRouteTag,
        OvertimeApprovePostRouteTag, OvertimeDetailRouteTag, OvertimeEditPostRouteTag,
        OvertimeListRouteTag, OvertimeRejectPostRouteTag, OvertimeRevokeApprovalPostRouteTag,
        OvertimeRevokeRejectionPostRouteTag,
    },
    state::HrState,
    templates::overtime::{
        ApprovedOvertimeCreateModalPage, ApprovedOvertimeListPage, ApprovedOvertimeRow,
        OvertimeApproveModalPage, OvertimeCreateModalPage, OvertimeDeleteModalPage,
        OvertimeDetailPage, OvertimeEditModalPage, OvertimeListPage, OvertimeRejectModalPage,
        OvertimeRevokeApprovalModalPage, OvertimeRevokeRejectionModalPage, OvertimeRow,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct OvertimeListQuery {
    #[serde(default, rename = "UserID", alias = "UserId", alias = "user_id")]
    pub user_id: Option<String>,
    #[serde(default, rename = "StartTime", alias = "start_time")]
    pub start_time: Option<String>,
    #[serde(default, rename = "EndTime", alias = "end_time")]
    pub end_time: Option<String>,
    #[serde(default, rename = "Status", alias = "status")]
    pub status: Option<String>,
    #[serde(default, rename = "tab")]
    pub tab: Option<String>,
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

fn hub_query_from_uri(uri: &Uri) -> OvertimeListQuery {
    let Some(query) = uri.query() else {
        return OvertimeListQuery::default();
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
        .column(approved_overtime::Column::OvertimeApplicationId)
        .from(approved_overtime::Entity)
        .to_owned()
}

fn rejected_id_subquery() -> SelectStatement {
    SeaQuery::select()
        .column(rejected_overtime::Column::OvertimeApplicationId)
        .from(rejected_overtime::Entity)
        .to_owned()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OvertimeIndex {
    /// Every application. Superuser only; other roles are sent to their own overtime.
    Directory,
    /// The signed-in user's applications, in every status.
    Applications,
    /// Pending applications of employees who report to the signed-in user.
    Queue,
}

impl OvertimeIndex {
    fn locked_status(self) -> Option<&'static str> {
        match self {
            Self::Directory | Self::Applications => None,
            Self::Queue => Some(FILTER_PENDING),
        }
    }

    fn personal(self) -> bool {
        matches!(self, Self::Applications)
    }

    fn title(self) -> &'static str {
        match self {
            Self::Directory | Self::Applications => "Overtime",
            Self::Queue => "Approve overtime",
        }
    }

    fn menu(self) -> &'static str {
        match self {
            Self::Directory | Self::Applications => "overtime",
            Self::Queue => "overtime-queue",
        }
    }

    fn filter_path(self) -> String {
        match self {
            Self::Directory => OvertimeListRouteTag.path(),
            Self::Applications => OvertimeApplicationsRouteTag.path(),
            Self::Queue => OvertimeApprovalsRouteTag.path(),
        }
    }
}

pub async fn list(
    state: Cap<HrState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    index(OvertimeIndex::Directory, state, chrome, ctx, htmx, uri).await
}

pub async fn applications(
    state: Cap<HrState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    index(OvertimeIndex::Applications, state, chrome, ctx, htmx, uri).await
}

pub async fn approvals(
    state: Cap<HrState>,
    chrome: Cap<SharedChromeFolder>,
    ctx: RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    index(OvertimeIndex::Queue, state, chrome, ctx, htmx, uri).await
}

async fn index(
    scope: OvertimeIndex,
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    if scope == OvertimeIndex::Directory && !Superuser::matches(&ctx.role) {
        return Redirect::to(&OvertimeApplicationsRouteTag.url()).into_response();
    }
    if scope == OvertimeIndex::Queue && !user_is_leave_manager(&state.db, ctx.user.id).await {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    }
    let q = hub_query_from_uri(&uri);
    if scope != OvertimeIndex::Queue && wants_approved_section(&q) {
        return Redirect::to(&OvertimeApprovedRouteTag.url()).into_response();
    }
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let mut query =
        scope_allowed::<super::super::routes::OvertimeView, _>(OvertimeApplicationEntity::find());
    if scope.personal() {
        query = query.filter(overtime_application::Column::UserId.eq(ctx.user.id));
    } else if scope == OvertimeIndex::Queue {
        let reports = managed_applicant_ids(&state.db, ctx.user.id)
            .await
            .unwrap_or_default();
        if reports.is_empty() {
            query = query.filter(Expr::cust("1 = 0"));
        } else {
            query = query.filter(overtime_application::Column::UserId.is_in(reports));
        }
        if let Some(id) = parse_user_id(q.user_id.as_deref()) {
            query = query.filter(overtime_application::Column::UserId.eq(id));
        }
    } else if let Some(id) = parse_user_id(q.user_id.as_deref()) {
        query = query.filter(overtime_application::Column::UserId.eq(id));
    }
    if let Some(start) = q
        .start_time
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| ctx.parse_datetime_local_input(s))
    {
        query = query.filter(overtime_application::Column::StartTime.gte(start));
    }
    if let Some(end) = q
        .end_time
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| ctx.parse_datetime_local_input(s))
    {
        query = query.filter(overtime_application::Column::EndTime.lte(end));
    }
    if let Some(reason) = q.reason.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(lariv_core::db::trigram::ci_contains(
            overtime_application::Column::Reason,
            reason,
        ));
    }
    let status = list_tab(scope, &q);
    query = match status {
        FILTER_APPROVED => {
            query.filter(overtime_application::Column::Id.in_subquery(approved_id_subquery()))
        }
        FILTER_REJECTED => query
            .filter(overtime_application::Column::Id.in_subquery(rejected_id_subquery()))
            .filter(overtime_application::Column::Id.not_in_subquery(approved_id_subquery())),
        FILTER_PENDING => query
            .filter(overtime_application::Column::Id.not_in_subquery(approved_id_subquery()))
            .filter(overtime_application::Column::Id.not_in_subquery(rejected_id_subquery())),
        _ => query,
    };
    let sort = q.sort.as_deref().unwrap_or("");
    let desc = sort_desc(sort);
    query = match sort.split_whitespace().next().unwrap_or("") {
        s if s.eq_ignore_ascii_case("User") => {
            if desc {
                query.order_by_desc(overtime_application::Column::UserId)
            } else {
                query.order_by_asc(overtime_application::Column::UserId)
            }
        }
        s if s.eq_ignore_ascii_case("End") => {
            if desc {
                query.order_by_desc(overtime_application::Column::EndTime)
            } else {
                query.order_by_asc(overtime_application::Column::EndTime)
            }
        }
        s if s.eq_ignore_ascii_case("Start") => {
            if desc {
                query.order_by_desc(overtime_application::Column::StartTime)
            } else {
                query.order_by_asc(overtime_application::Column::StartTime)
            }
        }
        _ => query
            .order_by_desc(overtime_application::Column::StartTime)
            .order_by_desc(overtime_application::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let ids: Vec<i64> = models.iter().map(|row| row.id).collect();
    let user_ids: Vec<i64> = models.iter().map(|row| row.user_id).collect();
    let names = user_display_labels(&state.db, &user_ids).await;
    let approved = approved_application_ids(&state.db, &ids).await;
    let rejected = rejected_application_ids(&state.db, &ids).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let user = names
                .get(&row.user_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", row.user_id));
            OvertimeRow {
                id: row.id,
                user,
                start_time: ctx.format_datetime(row.start_time).into_string(),
                end_time: ctx.format_datetime(row.end_time).into_string(),
                status: status_label(approved.contains(&row.id), rejected.contains(&row.id))
                    .to_string(),
                reason: row.reason.unwrap_or_default(),
                detail_href: OvertimeDetailRouteTag::new(row.id).url(),
            }
        })
        .collect();
    let filter_user_id = if scope.personal() {
        0
    } else {
        parse_user_id(q.user_id.as_deref()).unwrap_or(0)
    };
    let page = OvertimeListPage {
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        filter_user_id: fk_value(filter_user_id),
        filter_user_display: user_display_label(&state.db, filter_user_id).await,
        filter_start_time: q.start_time.unwrap_or_default(),
        filter_end_time: q.end_time.unwrap_or_default(),
        filter_reason: q.reason.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
        title: scope.title().to_string(),
        menu_active: scope.menu().to_string(),
        show_create: matches!(
            scope,
            OvertimeIndex::Directory | OvertimeIndex::Applications
        ) && status == FILTER_PENDING,
        show_user_filter: !scope.personal(),
        show_tabs: scope != OvertimeIndex::Queue,
        tab: status.to_string(),
        filter_path: scope.filter_path(),
        approved_href: OvertimeApprovedRouteTag.url(),
    };
    if htmx.targets::<OvertimeTableKey>() {
        return page.render_table().into_response();
    }
    crate::nav::hr_page(&state.db, ctx.user.id, || {
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response()
    })
    .await
}

pub async fn approved(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> Response {
    let q = hub_query_from_uri(&uri);
    let page_num = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.get();
    let superuser = Superuser::matches(&ctx.role);
    let mut query =
        scope_allowed::<super::super::routes::OvertimeView, _>(ApprovedOvertimeEntity::find());
    if superuser {
        if let Some(id) = parse_user_id(q.user_id.as_deref()) {
            query = query.filter(approved_overtime::Column::UserId.eq(id));
        }
    } else {
        query = query.filter(approved_overtime::Column::UserId.eq(ctx.user.id));
    }
    if let Some(start) = q
        .start_time
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| ctx.parse_datetime_local_input(s))
    {
        query = query.filter(approved_overtime::Column::StartTime.gte(start));
    }
    if let Some(end) = q
        .end_time
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(|s| ctx.parse_datetime_local_input(s))
    {
        query = query.filter(approved_overtime::Column::EndTime.lte(end));
    }
    let sort = q.sort.as_deref().unwrap_or("");
    let desc = sort_desc(sort);
    query = match sort.split_whitespace().next().unwrap_or("") {
        s if s.eq_ignore_ascii_case("User") => {
            if desc {
                query.order_by_desc(approved_overtime::Column::UserId)
            } else {
                query.order_by_asc(approved_overtime::Column::UserId)
            }
        }
        s if s.eq_ignore_ascii_case("End") => {
            if desc {
                query.order_by_desc(approved_overtime::Column::EndTime)
            } else {
                query.order_by_asc(approved_overtime::Column::EndTime)
            }
        }
        s if s.eq_ignore_ascii_case("ApprovedAt") => {
            if desc {
                query.order_by_desc(approved_overtime::Column::ApprovedAt)
            } else {
                query.order_by_asc(approved_overtime::Column::ApprovedAt)
            }
        }
        s if s.eq_ignore_ascii_case("Start") => {
            if desc {
                query.order_by_desc(approved_overtime::Column::StartTime)
            } else {
                query.order_by_asc(approved_overtime::Column::StartTime)
            }
        }
        _ => query
            .order_by_desc(approved_overtime::Column::StartTime)
            .order_by_desc(approved_overtime::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut user_ids: Vec<i64> = models.iter().map(|row| row.user_id).collect();
    user_ids.extend(models.iter().map(|row| row.approved_by_id));
    let names = user_display_labels(&state.db, &user_ids).await;
    let rows = models
        .into_iter()
        .map(|row| {
            let user = names
                .get(&row.user_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", row.user_id));
            let approved_by = names
                .get(&row.approved_by_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", row.approved_by_id));
            ApprovedOvertimeRow {
                user,
                start_time: ctx.format_datetime(row.start_time).into_string(),
                end_time: ctx.format_datetime(row.end_time).into_string(),
                approved_by,
                approved_at: ctx.format_datetime(row.approved_at).into_string(),
                detail_href: row
                    .overtime_application_id
                    .filter(|id| *id > 0)
                    .map(|id| OvertimeDetailRouteTag::new(id).url()),
            }
        })
        .collect();
    let filter_user_id = if superuser {
        parse_user_id(q.user_id.as_deref()).unwrap_or(0)
    } else {
        0
    };
    let page = ApprovedOvertimeListPage {
        rows: ObjectList::from_page(rows, page_num, page_size, total),
        filter_user_id: fk_value(filter_user_id),
        filter_user_display: user_display_label(&state.db, filter_user_id).await,
        filter_start_time: q.start_time.unwrap_or_default(),
        filter_end_time: q.end_time.unwrap_or_default(),
        sort: q.sort.unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
        show_user_filter: superuser,
        show_create: superuser,
        section_base: overtime_fallback(&ctx),
    };
    if htmx.targets::<ApprovedOvertimeTableKey>() {
        return page.render_table().into_response();
    }
    crate::nav::hr_page(&state.db, ctx.user.id, || {
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response()
    })
    .await
}

fn wants_approved_section(q: &OvertimeListQuery) -> bool {
    q.tab.as_deref() == Some(FILTER_APPROVED) || q.status.as_deref() == Some(FILTER_APPROVED)
}

fn list_tab(scope: OvertimeIndex, q: &OvertimeListQuery) -> &'static str {
    if let Some(locked) = scope.locked_status() {
        return locked;
    }
    let raw = q
        .tab
        .as_deref()
        .filter(|value| !value.is_empty())
        .or(q.status.as_deref())
        .unwrap_or(FILTER_PENDING);
    match raw {
        FILTER_APPROVED => FILTER_APPROVED,
        FILTER_REJECTED => FILTER_REJECTED,
        _ => FILTER_PENDING,
    }
}

fn overtime_fallback(ctx: &AuthContext) -> String {
    if Superuser::matches(&ctx.role) {
        OvertimeListRouteTag.url()
    } else {
        OvertimeApplicationsRouteTag.url()
    }
}

fn overtime_tab_href(base: &str, status: &str) -> String {
    if status == STATUS_APPROVED {
        return OvertimeApprovedRouteTag.url();
    }
    let tab = if status == STATUS_REJECTED {
        FILTER_REJECTED
    } else {
        FILTER_PENDING
    };
    format!("{base}?tab={tab}")
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let mut page = OvertimeCreateModalPage::new(q.form_name(), q.refresh_table());
    page.show_user = Superuser::matches(&ctx.role);
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<OvertimeApplicationForm>,
) -> Response {
    let user_id = match user_id_for_write(&ctx, &form, ctx.user.id) {
        Ok(id) => id,
        Err(e) => return create_error_response(&state, &chrome, &ctx, &q, &form, e).await,
    };
    let input = match application_input_from_form(&ctx, user_id, &form) {
        Ok(input) => input,
        Err(e) => return create_error_response(&state, &chrome, &ctx, &q, &form, e).await,
    };
    match create_overtime_application(&state.db, input).await {
        Ok(row) => respond_create_modal_done::<OvertimeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &OvertimeDetailRouteTag::new(row.id).url(),
        ),
        Err(e) => create_error_response(&state, &chrome, &ctx, &q, &form, e).await,
    }
}

pub async fn approved_create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let page = ApprovedOvertimeCreateModalPage::new(q.form_name(), q.refresh_table());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn approved_create_post(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ApprovedOvertimeForm>,
) -> Response {
    let input = match approved_input_from_form(&ctx, &form) {
        Ok(input) => input,
        Err(e) => {
            return approved_create_error(&state, &chrome, &ctx, &q, &form, e).await;
        }
    };
    match create_approved_overtime(&state.db, input).await {
        Ok(_) => respond_create_modal_done::<ApprovedOvertimeCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &OvertimeApprovedRouteTag.url(),
        ),
        Err(e) => approved_create_error(&state, &chrome, &ctx, &q, &form, e).await,
    }
}

pub async fn detail(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    let page = detail_page(&state.db, &ctx, row).await;
    crate::nav::hr_page(&state.db, ctx.user.id, || {
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response()
    })
    .await
}

pub async fn edit_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if ensure_overtime_editor(&ctx, row.user_id).is_err() || !is_pending(&state.db, id).await {
        return Redirect::to(&OvertimeDetailRouteTag::new(id).url()).into_response();
    }
    let show_user = Superuser::matches(&ctx.role);
    let page = OvertimeEditModalPage {
        id: row.id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeEditPostRouteTag::new(row.id), &q.form_name()),
        user_id: fk_value(row.user_id),
        user_display: user_display_label(&state.db, row.user_id).await,
        show_user,
        start_time: ctx.datetime_local_input(row.start_time).into_string(),
        end_time: ctx.datetime_local_input(row.end_time).into_string(),
        reason: row.reason.unwrap_or_default(),
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
    HtmlFormBody(form): HtmlFormBody<OvertimeApplicationForm>,
) -> Response {
    let Some(existing) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if let Err(e) = ensure_overtime_editor(&ctx, existing.user_id) {
        return edit_error_response(&state, &chrome, &ctx, id, &q, &form, e).await;
    }
    let user_id = match user_id_for_write(&ctx, &form, existing.user_id) {
        Ok(user_id) => user_id,
        Err(e) => return edit_error_response(&state, &chrome, &ctx, id, &q, &form, e).await,
    };
    let input = match application_input_from_form(&ctx, user_id, &form) {
        Ok(input) => input,
        Err(e) => return edit_error_response(&state, &chrome, &ctx, id, &q, &form, e).await,
    };
    match update_overtime_application(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<OvertimeEditModalKey>(
            &htmx,
            &OvertimeDetailRouteTag::new(id).url(),
        ),
        Err(e) => edit_error_response(&state, &chrome, &ctx, id, &q, &form, e).await,
    }
}

pub async fn delete_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if ensure_overtime_editor(&ctx, row.user_id).is_err() || !is_pending(&state.db, id).await {
        return Redirect::to(&OvertimeDetailRouteTag::new(id).url()).into_response();
    }
    let page = OvertimeDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this overtime application?".into(),
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
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if let Err(e) = ensure_overtime_editor(&ctx, row.user_id) {
        return delete_error_response(&chrome, &ctx, id, &q, e).await;
    }
    match delete_overtime_application(&state.db, id).await {
        Ok(()) => htmx.redirect(&overtime_fallback(&ctx)),
        Err(e) => delete_error_response(&chrome, &ctx, id, &q, e).await,
    }
}

pub async fn approve_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if !is_pending(&state.db, id).await
        || ensure_overtime_approver(&state.db, &ctx, row.user_id)
            .await
            .is_err()
    {
        return Redirect::to(&OvertimeDetailRouteTag::new(id).url()).into_response();
    }
    let page = OvertimeApproveModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeApprovePostRouteTag::new(id), &q.form_name()),
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
    HtmlFormBody(_form): HtmlFormBody<ApproveOvertimeForm>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if let Err(e) = ensure_overtime_approver(&state.db, &ctx, row.user_id).await {
        return approve_error_response(&chrome, &ctx, id, &q, e).await;
    }
    let input = ApproveOvertimeInput {
        approved_by_id: ctx.user.id,
        approved_at: Utc::now(),
    };
    match approve_overtime(&state.db, id, &ctx, input).await {
        Ok(_) => respond_edit_modal_done::<OvertimeApproveModalKey>(
            &htmx,
            &OvertimeDetailRouteTag::new(id).url(),
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
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if !is_pending(&state.db, id).await
        || ensure_overtime_approver(&state.db, &ctx, row.user_id)
            .await
            .is_err()
    {
        return Redirect::to(&OvertimeDetailRouteTag::new(id).url()).into_response();
    }
    let page = OvertimeRejectModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeRejectPostRouteTag::new(id), &q.form_name()),
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
    HtmlFormBody(_form): HtmlFormBody<RejectOvertimeForm>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if let Err(e) = ensure_overtime_approver(&state.db, &ctx, row.user_id).await {
        return reject_error_response(&chrome, &ctx, id, &q, e).await;
    }
    let input = RejectOvertimeInput {
        rejected_by_id: ctx.user.id,
        rejected_at: Utc::now(),
    };
    match reject_overtime(&state.db, id, &ctx, input).await {
        Ok(_) => respond_edit_modal_done::<OvertimeRejectModalKey>(
            &htmx,
            &OvertimeDetailRouteTag::new(id).url(),
        ),
        Err(e) => reject_error_response(&chrome, &ctx, id, &q, e).await,
    }
}

pub async fn revoke_approval_get(
    Cap(state): Cap<HrState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if find_approval(&state.db, id).await.is_none()
        || ensure_overtime_approver(&state.db, &ctx, row.user_id)
            .await
            .is_err()
    {
        return Redirect::to(&OvertimeDetailRouteTag::new(id).url()).into_response();
    }
    let page = OvertimeRevokeApprovalModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeRevokeApprovalPostRouteTag::new(id), &q.form_name()),
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
    HtmlFormBody(_form): HtmlFormBody<RevokeOvertimeApprovalForm>,
) -> Response {
    match revoke_approval(&state.db, id, &ctx).await {
        Ok(()) => respond_edit_modal_done::<OvertimeRevokeApprovalModalKey>(
            &htmx,
            &OvertimeDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = OvertimeRevokeApprovalModalPage {
                id,
                form_name: q.form_name(),
                post_url: modal_edit_post_url(
                    OvertimeRevokeApprovalPostRouteTag::new(id),
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
    let Some(row) = find_overtime_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&overtime_fallback(&ctx)).into_response();
    };
    if find_rejection(&state.db, id).await.is_none()
        || ensure_overtime_approver(&state.db, &ctx, row.user_id)
            .await
            .is_err()
    {
        return Redirect::to(&OvertimeDetailRouteTag::new(id).url()).into_response();
    }
    let page = OvertimeRevokeRejectionModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeRevokeRejectionPostRouteTag::new(id), &q.form_name()),
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
    HtmlFormBody(_form): HtmlFormBody<RevokeOvertimeRejectionForm>,
) -> Response {
    match revoke_rejection(&state.db, id, &ctx).await {
        Ok(()) => respond_edit_modal_done::<OvertimeRevokeRejectionModalKey>(
            &htmx,
            &OvertimeDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = OvertimeRevokeRejectionModalPage {
                id,
                form_name: q.form_name(),
                post_url: modal_edit_post_url(
                    OvertimeRevokeRejectionPostRouteTag::new(id),
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
    row: overtime_application::Model,
) -> OvertimeDetailPage {
    let user = user_display_label(db, row.user_id).await;
    let can_decide = ensure_overtime_approver(db, ctx, row.user_id).await.is_ok();
    let approved = find_approval(db, row.id).await;
    let rejected = find_rejection(db, row.id).await;
    let pending = approved.is_none() && rejected.is_none();
    let can_edit = pending && ensure_overtime_editor(ctx, row.user_id).is_ok();
    let approved_by = match &approved {
        Some(decision) => user_display_label(db, decision.approved_by_id).await,
        None => String::new(),
    };
    let rejected_by = match &rejected {
        Some(decision) => user_display_label(db, decision.rejected_by_id).await,
        None => String::new(),
    };
    let status = status_label(approved.is_some(), rejected.is_some());
    let own = row.user_id == ctx.user.id;
    let (menu_active, parent_label, parent_href) = if Superuser::matches(&ctx.role) {
        (
            "overtime",
            "Overtime",
            overtime_tab_href(&OvertimeListRouteTag.url(), status),
        )
    } else if !own {
        (
            "overtime-queue",
            "Approve overtime",
            OvertimeApprovalsRouteTag.url(),
        )
    } else {
        (
            "overtime",
            "Overtime",
            overtime_tab_href(&OvertimeApplicationsRouteTag.url(), status),
        )
    };
    OvertimeDetailPage {
        id: row.id,
        title: user.clone(),
        user,
        start_time: ctx.format_datetime(row.start_time).into_string(),
        end_time: ctx.format_datetime(row.end_time).into_string(),
        reason: row.reason.unwrap_or_default(),
        status: status.to_string(),
        approved_by,
        approved_at: approved
            .as_ref()
            .map(|decision| ctx.format_datetime(decision.approved_at).into_string())
            .unwrap_or_default(),
        rejected_by,
        rejected_at: rejected
            .as_ref()
            .map(|decision| ctx.format_datetime(decision.rejected_at).into_string())
            .unwrap_or_default(),
        can_approve: pending && can_decide,
        can_edit,
        can_revoke_approval: approved.is_some() && can_decide,
        can_revoke_rejection: rejected.is_some() && can_decide,
        menu_active: menu_active.to_string(),
        parent_label: parent_label.to_string(),
        parent_href,
    }
}

/// Superuser files overtime for the selected user and may use any times.
/// Everyone else files their own, limited to today or yesterday.
fn user_id_for_write(
    ctx: &AuthContext,
    form: &OvertimeApplicationForm,
    fallback: i64,
) -> Result<i64, String> {
    if Superuser::matches(&ctx.role) {
        if form.user_id <= 0 {
            return Err("user is required".to_string());
        }
        return Ok(form.user_id);
    }
    Ok(fallback)
}

fn application_input_from_form(
    ctx: &AuthContext,
    user_id: i64,
    form: &OvertimeApplicationForm,
) -> Result<OvertimeApplicationInput, String> {
    if user_id <= 0 {
        return Err("user is required".to_string());
    }
    Ok(OvertimeApplicationInput {
        user_id,
        start_time: parse_required_datetime(ctx, &form.start_time, "start")?,
        end_time: parse_required_datetime(ctx, &form.end_time, "end")?,
        reason: optional_reason(&form.reason),
        timezone: ctx.timezone.clone(),
        enforce_recent: !Superuser::matches(&ctx.role),
    })
}

fn approved_input_from_form(
    ctx: &AuthContext,
    form: &ApprovedOvertimeForm,
) -> Result<ApprovedOvertimeInput, String> {
    if form.user_id <= 0 {
        return Err("user is required".to_string());
    }
    if form.approved_by_id <= 0 {
        return Err("approved by is required".to_string());
    }
    Ok(ApprovedOvertimeInput {
        user_id: form.user_id,
        start_time: parse_required_datetime(ctx, &form.start_time, "start")?,
        end_time: parse_required_datetime(ctx, &form.end_time, "end")?,
        approved_by_id: form.approved_by_id,
        approved_at: parse_required_datetime(ctx, &form.approved_at, "approved at")?,
    })
}

fn parse_required_datetime(
    ctx: &AuthContext,
    raw: &str,
    label: &str,
) -> Result<DateTime<Utc>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(format!("{label} is required"));
    }
    ctx.parse_datetime_local_input(raw)
        .ok_or_else(|| format!("invalid {label}"))
}

async fn create_error_response(
    state: &HrState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    form: &OvertimeApplicationForm,
    error: String,
) -> Response {
    let page = OvertimeCreateModalPage::with_form(
        q.form_name(),
        q.refresh_table(),
        form,
        user_display_label(&state.db, form.user_id).await,
        Superuser::matches(&ctx.role),
        error,
    );
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn approved_create_error(
    state: &HrState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    q: &ModalNameQuery,
    form: &ApprovedOvertimeForm,
    error: String,
) -> Response {
    let page = ApprovedOvertimeCreateModalPage::with_form(
        q.form_name(),
        q.refresh_table(),
        form,
        user_display_label(&state.db, form.user_id).await,
        user_display_label(&state.db, form.approved_by_id).await,
        error,
    );
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn edit_error_response(
    state: &HrState,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &OvertimeApplicationForm,
    error: String,
) -> Response {
    let page = OvertimeEditModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeEditPostRouteTag::new(id), &q.form_name()),
        user_id: fk_value(form.user_id),
        user_display: user_display_label(&state.db, form.user_id).await,
        show_user: Superuser::matches(&ctx.role),
        start_time: form.start_time.clone(),
        end_time: form.end_time.clone(),
        reason: form.reason.clone(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn delete_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    error: String,
) -> Response {
    let page = OvertimeDeleteModalPage {
        id,
        form_name: q.form_name(),
        message: "Are you sure you want to delete this overtime application?".into(),
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
    let page = OvertimeApproveModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeApprovePostRouteTag::new(id), &q.form_name()),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn reject_error_response(
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    error: String,
) -> Response {
    let page = OvertimeRejectModalPage {
        id,
        form_name: q.form_name(),
        post_url: modal_edit_post_url(OvertimeRejectPostRouteTag::new(id), &q.form_name()),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn is_pending(db: &sea_orm::DatabaseConnection, id: i64) -> bool {
    find_approval(db, id).await.is_none() && find_rejection(db, id).await.is_none()
}

async fn find_overtime_scoped(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    ctx: &AuthContext,
) -> Option<overtime_application::Model> {
    let row = lariv_core::web::opt_or_log(
        scope_allowed::<super::super::routes::OvertimeView, _>(
            OvertimeApplicationEntity::find_by_id(id),
        )
        .one(db)
        .await,
        "find overtime application by id",
    )?;
    let manager_id = applicant_manager_id(db, row.user_id).await.ok()?;
    actor_may_view_leave(&ctx.role, ctx.user.id, row.user_id, manager_id).then_some(row)
}

#[cfg(test)]
mod tests {
    use super::OvertimeListQuery;
    use lariv_core::html_form::UrlencodedFields;

    #[test]
    fn filter_query_accepts_the_form_field_names() {
        let q: OvertimeListQuery = UrlencodedFields::parse(
            b"UserID=4&StartTime=2026-10-09T09:00&EndTime=2026-10-09T18:00&Status=pending&Reason=ship",
        )
        .unwrap()
        .deserialize()
        .unwrap();
        assert_eq!(q.user_id.as_deref(), Some("4"));
        assert_eq!(q.start_time.as_deref(), Some("2026-10-09T09:00"));
        assert_eq!(q.end_time.as_deref(), Some("2026-10-09T18:00"));
        assert_eq!(q.status.as_deref(), Some("pending"));
        assert_eq!(q.reason.as_deref(), Some("ship"));
    }
}
