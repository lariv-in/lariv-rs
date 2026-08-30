use axum::{extract::Query, http::Uri};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::template::RenderAppPane;
use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx},
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
    web::{Htmx, QueryPageSize, html_built_page_with_slots},
};

#[derive(Debug, Deserialize, Default)]
pub struct HistoryListQuery {
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

/// Collapse whitespace and truncate for use as a session title.
pub fn title_from_first_prompt(prompt: &str) -> String {
    let collapsed: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return String::new();
    }
    const MAX: usize = 72;
    if collapsed.chars().count() <= MAX {
        collapsed
    } else {
        let mut truncated: String = collapsed.chars().take(MAX.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    }
}

/// If the session has no title yet, set it from the first prompt. Returns the new title when set.
pub async fn maybe_set_session_title_from_prompt(
    db: &sea_orm::DatabaseConnection,
    session_id: i64,
    prompt: &str,
) -> Result<Option<String>, String> {
    let title = title_from_first_prompt(prompt);
    if title.is_empty() {
        return Ok(None);
    }
    let sess = SessionEntity::find_by_id(session_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "session not found".to_string())?;
    if !sess.title.trim().is_empty() {
        return Ok(None);
    }
    let mut am: session::ActiveModel = sess.into();
    am.title = Set(title.clone());
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(Some(title))
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
    let paginator = query.paginate(db, q.page_size.get() as u64);
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
    ObjectList::from_page(rows, page, q.page_size.get(), total)
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
        page_size: q.page_size.get(),
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

#[cfg(test)]
mod tests {
    use super::{session_display_title, title_from_first_prompt};

    #[test]
    fn title_from_prompt_collapses_whitespace() {
        assert_eq!(title_from_first_prompt("  hello   world  "), "hello world");
    }

    #[test]
    fn title_from_prompt_truncates_long_text() {
        let long = "a".repeat(100);
        let title = title_from_first_prompt(&long);
        assert_eq!(title.chars().count(), 72);
        assert!(title.ends_with('…'));
    }

    #[test]
    fn title_from_prompt_empty() {
        assert!(title_from_first_prompt("   ").is_empty());
    }

    #[test]
    fn display_title_falls_back_when_empty() {
        assert_eq!(session_display_title(9, ""), "Session #9");
        assert_eq!(session_display_title(9, "  Hello  "), "Hello");
    }
}
