use axum::{extract::Query, http::Uri};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
};
use serde::Deserialize;

use crate::{
    entities::session::{self, Entity as SessionEntity},
    keys::{HISTORY_MODAL_TABLE, HISTORY_TABLE},
    state::LlmAssistantState,
    templates::{HistoryListPage, HistoryRow},
};
use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{Htmx, QueryPage, QueryPageSize, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;

#[derive(Debug, Deserialize, Default)]
pub struct HistoryListQuery {
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

pub fn format_updated_at(dt: Option<chrono::DateTime<Utc>>, tz: &str) -> String {
    lariv_core::datetime::DatetimeLabel::short_optional(dt, tz).into_string()
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

/// Title cell for the history table.
pub fn session_list_title(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        "(untitled)".to_string()
    } else {
        title.to_string()
    }
}

pub async fn load_history_page(
    db: &sea_orm::DatabaseConnection,
    user_id: i64,
    role: &str,
    q: &HistoryListQuery,
    tz: &str,
) -> ObjectList<HistoryRow> {
    let mut query = SessionEntity::find();
    if !lariv_plugin_users::roles::Superuser::matches(role) {
        query = query.filter(session::Column::UserId.eq(user_id));
    }
    let sort = q.sort.as_deref().unwrap_or("").trim();
    let query = match sort {
        s if s.eq_ignore_ascii_case("Title DESC") => query.order_by_desc(session::Column::Title),
        s if s.eq_ignore_ascii_case("Title ASC") || s.eq_ignore_ascii_case("Title") => {
            query.order_by_asc(session::Column::Title)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt DESC") => {
            query.order_by_desc(session::Column::UpdatedAt)
        }
        s if s.eq_ignore_ascii_case("UpdatedAt ASC") || s.eq_ignore_ascii_case("UpdatedAt") => {
            query.order_by_asc(session::Column::UpdatedAt)
        }
        s if s.eq_ignore_ascii_case("Id ASC") => query.order_by_asc(session::Column::Id),
        _ => query.order_by_desc(session::Column::Id),
    };
    let page = q.page.get();
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
                title: session_list_title(&s.title),
                updated_at: updated,
                is_subagent: s.is_subagent,
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
    let sessions =
        load_history_page(&state.db, ctx.user.id, ctx.role.as_str(), &q, &ctx.timezone).await;
    let page = HistoryListPage {
        sessions,
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
        sort: q.sort.clone().unwrap_or_default(),
    };
    if htmx.targets(HISTORY_MODAL_TABLE) {
        return page.render_modal_table();
    }
    if htmx.targets(HISTORY_TABLE) {
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
    use super::{session_display_title, session_list_title, title_from_first_prompt};

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
    fn list_title_uses_untitled_when_empty() {
        assert_eq!(session_list_title("  Hello  "), "Hello");
        assert_eq!(session_list_title("   "), "(untitled)");
    }

    #[test]
    fn display_title_falls_back_when_empty() {
        assert_eq!(session_display_title(9, ""), "Session #9");
        assert_eq!(session_display_title(9, "  Hello  "), "Hello");
    }
}
