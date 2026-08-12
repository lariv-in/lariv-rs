use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots},
    template::RenderAppPane,
};

use crate::plugins::finance_accounts::entities::{journal_entry, JournalEntryEntity};

use crate::plugins::finance_creditnotes::{
    entities::credit_note::{self, Entity as CreditNoteEntity},
    keys::CreditNoteTableKey,
    scope::{find_credit_note_scoped, scope_credit_notes},
    state::CreditnotesState,
    templates::{CreditNoteDetailPage, CreditNoteListPage, CreditNoteRow},
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: Option<u32>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}


fn journal_entry_datetime_label(entry: &journal_entry::Model, tz: &str) -> String {
    crate::datetime::DatetimeLabel::short(entry.datetime, tz).into_string()
}

async fn load_journal_entry_labels(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
    tz: &str,
) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    JournalEntryEntity::find()
        .filter(journal_entry::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|e| (e.id, journal_entry_datetime_label(&e, tz)))
        .collect()
}

async fn query_rows(
    db: &sea_orm::DatabaseConnection,
    auth: &AuthContext,
    page: u32,
    sort: Option<&str>,
) -> (Vec<CreditNoteRow>, u32, u64) {
    let mut query = scope_credit_notes(CreditNoteEntity::find(), auth);
    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Date DESC") => query.order_by_desc(credit_note::Column::Datetime),
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(credit_note::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Reason DESC") => query.order_by_desc(credit_note::Column::Reason),
        s if s.eq_ignore_ascii_case("Reason ASC") || s.eq_ignore_ascii_case("Reason") => {
            query.order_by_asc(credit_note::Column::Reason)
        }
        _ => query
            .order_by_desc(credit_note::Column::Datetime)
            .order_by_desc(credit_note::Column::Id),
    };
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut entry_ids = Vec::new();
    for c in &models {
        entry_ids.push(c.journal_entry_id);
        entry_ids.push(c.reversed_journal_entry_id);
    }
    entry_ids.sort_unstable();
    entry_ids.dedup();
    let entry_labels = load_journal_entry_labels(db, &entry_ids, &auth.timezone).await;
    let rows = models
        .into_iter()
        .map(|c| CreditNoteRow {
            id: c.id,
            datetime: auth.format_datetime_seconds(c.datetime).into_string(),
            reason: c.reason.unwrap_or_default(),
            original_entry_label: entry_labels
                .get(&c.journal_entry_id)
                .cloned()
                .unwrap_or_else(|| "—".into()),
            reversal_entry_label: entry_labels
                .get(&c.reversed_journal_entry_id)
                .cloned()
                .unwrap_or_else(|| "—".into()),
        })
        .collect();
    (rows, page, total)
}

pub async fn list(
    Cap(state): Cap<CreditnotesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let page_num = q.page.unwrap_or(1).max(1);
    let (rows, page, total) = query_rows(&state.db, &ctx, page_num, q.sort.as_deref()).await;
    let credit_notes = ObjectList::from_page(rows, page, PAGE_SIZE, total);
    let page = CreditNoteListPage {
        credit_notes,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<CreditNoteTableKey>() {
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

pub async fn detail(
    Cap(state): Cap<CreditnotesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(c) = find_credit_note_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-credit-notes/").into_response();
    };
    let page = CreditNoteDetailPage {
        id: c.id,
        datetime: ctx.format_datetime_seconds(c.datetime).into_string(),
        reason: c.reason.unwrap_or_default(),
        journal_entry_id: c.journal_entry_id,
        reversed_journal_entry_id: c.reversed_journal_entry_id,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
