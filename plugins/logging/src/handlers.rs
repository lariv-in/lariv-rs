//! HTTP handler for the audit log list.

use std::collections::HashMap;

use axum::extract::Query;
use axum::http::Uri;
use sea_orm::sea_query::{Alias, Condition, Expr, ExprTrait};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    FilterField, FilterRowError, apply_filters, choice_column, datetime_column, foreign_key_column,
    number_column, parse_list_filters, text_expr_column,
};
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};
use lariv_plugin_users::entities::user::{Column as UserColumn, Entity as UserEntity};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::routes::UsersSelectRouteTag;

use crate::entities::log::{Column as LogColumn, Entity as LogEntity, LogKind};
use crate::keys::LOG_TABLE;
use crate::routes::LogListRouteTag;
use crate::state::LoggingState;
use crate::templates::{LogListPage, LogRow};

/// One page of audit rows plus the filter bar that produced it.
pub struct LogListData {
    pub logs: ObjectList<LogRow>,
    pub fields: Vec<FilterField>,
    pub filters: Vec<FilterClauseView>,
    pub filter_errors: Vec<FilterRowError>,
    pub sort: String,
    pub page_size: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct LogListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn user_label(names: &HashMap<i64, String>, user_id: Option<i64>) -> String {
    let Some(user_id) = user_id else {
        return String::new();
    };
    names
        .get(&user_id)
        .cloned()
        .unwrap_or_else(|| user_id.to_string())
}

async fn user_names(db: &sea_orm::DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    match UserEntity::find()
        .filter(UserColumn::Id.is_in(ids.to_vec()))
        .all(db)
        .await
    {
        Ok(users) => users.into_iter().map(|user| (user.id, user.name)).collect(),
        Err(error) => {
            tracing::error!(error = %error, "find audit log users");
            HashMap::new()
        }
    }
}

fn kind_choices() -> Vec<(String, String)> {
    [LogKind::Create, LogKind::Update, LogKind::Delete]
        .into_iter()
        .map(|kind| {
            let label = kind.as_str().to_string();
            (label.clone(), label)
        })
        .collect()
}

async fn table_name_choices(db: &sea_orm::DatabaseConnection) -> Vec<(String, String)> {
    match LogEntity::find()
        .select_only()
        .column(LogColumn::TableName)
        .distinct()
        .order_by_asc(LogColumn::TableName)
        .into_tuple::<String>()
        .all(db)
        .await
    {
        Ok(names) => names
            .into_iter()
            .filter(|name| !name.is_empty())
            .map(|name| {
                let label = name.clone();
                (name, label)
            })
            .collect(),
        Err(error) => {
            tracing::error!(error = %error, "list audit log table names");
            Vec::new()
        }
    }
}

async fn log_filter_fields(db: &sea_orm::DatabaseConnection, tz: &str) -> Vec<FilterField> {
    let tables = table_name_choices(db).await;
    vec![
        datetime_column("created_at", "When", LogColumn::CreatedAt, tz),
        choice_column("table_name", "Table", LogColumn::TableName, tables),
        number_column("row_id", "Row", LogColumn::RowId),
        choice_column("log_kind", "Kind", LogColumn::LogKind, kind_choices()),
        foreign_key_column(
            "user_id",
            "User",
            LogColumn::UserId,
            UsersSelectRouteTag.url(),
            "Any user…",
        ),
        text_expr_column(
            "description",
            "Description",
            Expr::col(LogColumn::Description).cast_as(Alias::new("text")),
        ),
    ]
}

async fn fill_user_displays(db: &sea_orm::DatabaseConnection, views: &mut [FilterClauseView]) {
    let ids: Vec<i64> = views
        .iter()
        .filter(|view| view.clause.field == "user_id")
        .filter_map(|view| view.clause.value().parse().ok())
        .filter(|id| *id > 0)
        .collect();
    let names = user_names(db, &ids).await;
    for view in views.iter_mut() {
        if view.clause.field != "user_id" {
            continue;
        }
        let Ok(id) = view.clause.value().parse::<i64>() else {
            continue;
        };
        view.fk_display = user_label(&names, Some(id));
    }
}

fn normalized_sort(sort: Option<&str>) -> &'static str {
    match sort.map(str::trim).filter(|sort| !sort.is_empty()) {
        Some("CreatedAt ASC") => "CreatedAt ASC",
        Some("TableName DESC") => "TableName DESC",
        Some("TableName ASC" | "TableName") => "TableName ASC",
        Some("LogKind DESC") => "LogKind DESC",
        Some("LogKind ASC" | "LogKind") => "LogKind ASC",
        _ => "CreatedAt DESC",
    }
}

async fn load_page(
    db: &sea_orm::DatabaseConnection,
    q: &LogListQuery,
    sort: &str,
    tz: &str,
    condition: Condition,
) -> ObjectList<LogRow> {
    let mut query = LogEntity::find().filter(condition);
    query = match sort {
        "CreatedAt ASC" => query.order_by_asc(LogColumn::CreatedAt),
        "TableName DESC" => query.order_by_desc(LogColumn::TableName),
        "TableName ASC" => query.order_by_asc(LogColumn::TableName),
        "LogKind DESC" => query.order_by_desc(LogColumn::LogKind),
        "LogKind ASC" => query.order_by_asc(LogColumn::LogKind),
        _ => query.order_by_desc(LogColumn::CreatedAt),
    };

    let page = q.page.get();
    let paginator = query.paginate(db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let user_ids: Vec<i64> = models.iter().filter_map(|row| row.user_id).collect();
    let names = user_names(db, &user_ids).await;
    let rows = models
        .into_iter()
        .map(|row| LogRow {
            created_at: lariv_core::datetime::DatetimeLabel::short(row.created_at, tz)
                .into_string(),
            table_name: row.table_name,
            row_id: row.row_id.to_string(),
            log_kind: row.log_kind.as_str().to_string(),
            user: user_label(&names, row.user_id),
            description: row.description,
        })
        .collect();
    ObjectList::from_page(rows, page, q.page_size.get(), total)
}

/// Load one page of audit rows.
///
/// `extra` is always ANDed, so a caller can lock the page to one user.
/// Fields named in `omit_fields` are left out of the filter bar; a query string
/// that still names them is ignored.
pub async fn load_log_list(
    db: &sea_orm::DatabaseConnection,
    q: &LogListQuery,
    raw_query: &str,
    timezone: &str,
    extra: Condition,
    omit_fields: &[&str],
) -> LogListData {
    let sort = normalized_sort(q.sort.as_deref());
    let mut fields = log_filter_fields(db, timezone).await;
    if !omit_fields.is_empty() {
        fields.retain(|field| !omit_fields.contains(&field.key()));
    }
    let clauses = parse_list_filters(raw_query);
    let applied = apply_filters(db.get_database_backend(), &fields, &clauses);
    let mut filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    fill_user_displays(db, &mut filters).await;
    let condition = Condition::all().add(applied.condition).add(extra);
    let logs = load_page(db, q, sort, timezone, condition).await;
    LogListData {
        logs,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: sort.to_string(),
        page_size: q.page_size.get(),
    }
}

/// HTTP handler: list audit log rows.
pub async fn list(
    Cap(state): Cap<LoggingState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<LogListQuery>,
) -> maud::Markup {
    let data = load_log_list(
        &state.db,
        &q,
        uri.query().unwrap_or(""),
        &ctx.timezone,
        Condition::all(),
        &[],
    )
    .await;
    let page = LogListPage {
        logs: data.logs,
        fields: data.fields,
        filters: data.filters,
        filter_errors: data.filter_errors,
        sort: data.sort,
        path_and_query: path_and_query(&uri),
        page_size: data.page_size,
        filter_form_url: LogListRouteTag.url(),
    };
    if htmx.targets(LOG_TABLE) {
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
