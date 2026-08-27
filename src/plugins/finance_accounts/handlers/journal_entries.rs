use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde::Deserialize;

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, DEFAULT_PAGE_SIZE},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::middleware::RequireAuth,
    web::{
        html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done, Htmx,
        QueryPage,
    },
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::{
    entities::journal_entry::{self},
    forms::JournalEntryForm,
    handlers::ModalNameQuery,
    keys::{JournalEntryCreateModalKey, JournalEntrySelectModalKey, JournalEntrySelectTableKey},
    logic::journal::delete_journal_entry_recursive,
    routes::{JournalDetailRouteTag, JournalEntryDeleteGetRouteTag, JournalEntryDetailRouteTag},
    scope::{
        find_journal_entry_scoped, find_journal_scoped, journal_entry_sort,
        load_journal_currency_format, load_journal_entry_items, query_journal_entries_for_select,
    },
    source_doc_label::resolve_source_doc_datetime,
    source_doc_label::resolve_source_doc_display,
    source_doc_registry::SourceDocRegistry,
    state::AccountsState,
    templates::{
        JournalEntryCreateModalPage, JournalEntryDeletePage, JournalEntryDetailPage,
        JournalEntryItemRow, JournalEntryRow, JournalEntrySelectPage,
    },
};

use super::util::{parse_i64, path_and_query};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct JournalEntrySelectQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub target_input: Option<String>,
}

pub async fn create_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(journal_id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalDetailRouteTag::new(journal_id).url()).into_response();
    }
    let Some(journal) = find_journal_scoped(&state.db, journal_id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    let page = JournalEntryCreateModalPage::new(
        q.form_name(),
        q.refresh_table(),
        journal.id,
        journal.name,
    );
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Path(journal_id): Path<i64>,
    Form(form): Form<JournalEntryForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalDetailRouteTag::new(journal_id).url()).into_response();
    }
    let Some(journal) = find_journal_scoped(&state.db, journal_id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    let source_doc_id = parse_i64(&form.source_doc_id).unwrap_or(0);
    let Some(datetime) = resolve_source_doc_datetime(&state.db, &source_docs, source_doc_id).await
    else {
        let source_doc_display = if source_doc_id > 0 {
            resolve_source_doc_display(&state.db, &source_docs, source_doc_id)
                .await
                .summary_label()
        } else {
            String::new()
        };
        let page = JournalEntryCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            journal_id,
            journal_name: journal.name,
            source_doc_id: form.source_doc_id,
            source_doc_display,
            error: "Could not resolve source document timestamp".into(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let now = Utc::now();
    let model = journal_entry::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        datetime: Set(datetime),
        source_doc_id: Set(source_doc_id),
        journal_id: Set(journal_id),
        ..Default::default()
    };
    match model.insert(&state.db).await {
        Ok(_) => respond_create_modal_done::<JournalEntryCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &JournalDetailRouteTag::new(journal_id).url(),
        ),
        Err(e) => {
            let source_doc_display = if source_doc_id > 0 {
                resolve_source_doc_display(&state.db, &source_docs, source_doc_id)
                    .await
                    .summary_label()
            } else {
                String::new()
            };
            let page = JournalEntryCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                journal_id,
                journal_name: journal.name,
                source_doc_id: form.source_doc_id,
                source_doc_display,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(entry) = find_journal_entry_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    let journal = find_journal_scoped(&state.db, entry.journal_id, &ctx).await;
    let journal_mutable = journal.as_ref().map(|j| j.is_mutable).unwrap_or(false);
    let journal_name = journal
        .as_ref()
        .map(|j| j.name.clone())
        .unwrap_or_else(|| "—".into());
    let source_doc = resolve_source_doc_display(&state.db, &source_docs, entry.source_doc_id).await;
    let items_raw = load_journal_entry_items(&state.db, entry.id).await;
    let currency = load_journal_currency_format(&state.db, entry.journal_id).await;
    let items: Vec<JournalEntryItemRow> = items_raw
        .into_iter()
        .map(|(item, acct)| JournalEntryItemRow {
            datetime: ctx.format_datetime_seconds(item.datetime).into_string(),
            account_label: format!("{} — {}", acct.code, acct.name),
            amount: currency.display(item.amount),
        })
        .collect();
    let page = JournalEntryDetailPage {
        id: entry.id,
        datetime: ctx.format_datetime_seconds(entry.datetime).into_string(),
        journal_id: entry.journal_id,
        journal_label: format!("{journal_name} (#{})", entry.journal_id),
        source_doc_label: source_doc.type_label,
        source_doc_instance_name: source_doc.instance_name,
        source_doc_url: source_doc.detail_url,
        items,
        can_delete: require_superuser(&ctx) && journal_mutable,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn delete_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalEntryDetailRouteTag::new(id).url()).into_response();
    }
    let Some(entry) = find_journal_entry_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    let Some(journal) = find_journal_scoped(&state.db, entry.journal_id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    let page = JournalEntryDeletePage {
        id: entry.id,
        journal_id: entry.journal_id,
        journal_label: format!("{} (#{})", journal.name, entry.journal_id),
        can_delete: journal.is_mutable,
        error: if journal.is_mutable {
            None
        } else {
            Some("This journal is immutable. Enable Mutable on the journal edit page before deleting entries.".into())
        },
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalEntryDetailRouteTag::new(id).url()).into_response();
    }
    let Some(entry) = find_journal_entry_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    let Some(journal) = find_journal_scoped(&state.db, entry.journal_id, &ctx).await else {
        return Redirect::to("/finance/journals").into_response();
    };
    if !journal.is_mutable {
        return Redirect::to(&JournalEntryDeleteGetRouteTag::new(id).url()).into_response();
    }
    let journal_id = entry.journal_id;
    match delete_journal_entry_recursive(&state.db, entry.id).await {
        Ok(_) => Redirect::to(&JournalDetailRouteTag::new(journal_id).url()).into_response(),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete journal entry");
            let page = JournalEntryDeletePage {
                id: entry.id,
                journal_id: entry.journal_id,
                journal_label: format!("{} (#{})", journal.name, entry.journal_id),
                can_delete: journal.is_mutable,
                error: Some(e.to_string()),
            };
            html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<JournalEntrySelectQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance/journals").into_response();
    }
    let page = q.page.get();
    let (rows, total) =
        query_journal_entries_for_select(&state.db, &ctx, page, PAGE_SIZE, q.sort.as_deref()).await;
    let entries: Vec<JournalEntryRow> = rows
        .into_iter()
        .map(|(e, journal_name)| {
            let jn = journal_name.clone();
            JournalEntryRow {
                id: e.id,
                datetime: ctx.format_datetime_seconds(e.datetime).into_string(),
                source_doc_label: format!("entry #{}", e.id),
                source_doc_instance_name: String::new(),
                source_doc_url: String::new(),
                journal_name: jn.clone(),
                amount: String::new(),
                label: format!("{} · {}", jn, ctx.format_datetime_short(e.datetime)),
            }
        })
        .collect();
    let list = ObjectList::from_page(entries, page, PAGE_SIZE, total);
    let page = JournalEntrySelectPage {
        entries: list,
        target_input: q
            .target_input
            .unwrap_or_else(|| "JournalEntryID".to_string()),
        sort: journal_entry_sort(q.sort.as_deref()).to_string(),
        path_and_query: path_and_query(&uri),
    };
    respond_picker_select::<JournalEntrySelectTableKey, JournalEntrySelectModalKey, _>(&htmx, &page)
        .into_response()
}
