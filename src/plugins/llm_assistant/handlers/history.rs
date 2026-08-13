use axum::{extract::Query, http::Uri};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::template::RenderAppPane;
use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::{
        llm_assistant::{
            entities::session::{self, Entity as SessionEntity},
            keys::HistoryTableKey,
            state::LlmAssistantState,
            templates::{HistoryListPage, HistoryRow},
        },
        users::middleware::RequireAuth,
    },
    web::{Htmx, html_built_page_with_slots},
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct HistoryListQuery {
    #[serde(default)]
    pub page: Option<u32>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

pub fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
}

pub fn session_display_title(id: i64, title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        format!("Session #{id}")
    } else {
        title.to_string()
    }
}

pub fn session_label(id: i64, title: &str, updated_at: &str) -> String {
    let title = title.trim();
    let title = if title.is_empty() {
        "(untitled)"
    } else {
        title
    };
    if updated_at.is_empty() {
        format!("#{id} · {title}")
    } else {
        format!("#{id} · {title} · {updated_at}")
    }
}

/// All sessions for the user, newest first (sidebar modal).
pub async fn load_user_sessions(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    is_superuser: bool,
    tz: &str,
) -> Vec<(i64, String)> {
    let mut query = SessionEntity::find();
    if !is_superuser {
        query = query.filter(session::Column::UserId.eq(user_id));
    }
    let models = query
        .order_by_desc(session::Column::UpdatedAt)
        .all(db)
        .await
        .unwrap_or_default();
    models
        .into_iter()
        .map(|s| {
            let updated = format_updated_at(s.updated_at, tz);
            (s.id, session_label(s.id, &s.title, &updated))
        })
        .collect()
}

async fn load_history_page(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    is_superuser: bool,
    q: &HistoryListQuery,
    tz: &str,
) -> ObjectList<HistoryRow> {
    let mut query = SessionEntity::find();
    if !is_superuser {
        query = query.filter(session::Column::UserId.eq(user_id));
    }
    let query = query.order_by_desc(session::Column::UpdatedAt);
    let page = q.page.unwrap_or(1).max(1);
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|s| {
            let updated = format_updated_at(s.updated_at, tz);
            HistoryRow {
                id: s.id,
                label: session_label(s.id, &s.title, &updated),
            }
        })
        .collect();
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

/// HTTP handler: `list`.
pub async fn list(
    Cap(state): Cap<LlmAssistantState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<HistoryListQuery>,
) -> maud::Markup {
    let sessions = load_history_page(
        &state.db,
        ctx.user.id,
        ctx.user.is_superuser,
        &q,
        &ctx.timezone,
    )
    .await;
    let page = HistoryListPage {
        sessions,
        path_and_query: path_and_query(&uri),
    };
    if htmx.targets::<HistoryTableKey>() {
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
