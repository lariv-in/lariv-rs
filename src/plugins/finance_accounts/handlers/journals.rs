use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryOrder};
use serde::Deserialize;

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots, query_bool,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::{
    entities::journal::{self, Entity as JournalEntity},
    forms::{JournalCreateForm, JournalForm},
    handlers::ModalNameQuery,
    journal_type::JournalType,
    keys::{
        JournalCreateModalKey, JournalEditModalKey, JournalSelectModalKey, JournalSelectTableKey,
        JournalTableKey,
    },
    routes::{JournalDetailRouteTag, JournalListRouteTag},
    scope::{
        apply_journal_filters, currency_summary, find_journal_scoped, journal_entry_sort,
        load_currency_by_id, load_journal_entries_for_journal, load_journal_entry_transfer_amounts,
        scope_superuser,
    },
    source_doc_label::resolve_source_doc_display,
    source_doc_registry::SourceDocRegistry,
    state::AccountsState,
    templates::{
        JournalCreateModalPage, JournalDetailPage, JournalEditModalPage, JournalEntryRow,
        JournalListPage, JournalRow, JournalSelectPage,
    },
};

use super::util::{checkbox_on, parse_i64, path_and_query};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct JournalListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(
        default,
        rename = "IsActive",
        alias = "is_active",
        deserialize_with = "query_bool"
    )]
    pub is_active: Option<bool>,
    #[serde(default, rename = "CurrencyID", alias = "currency_id")]
    pub currency_id: Option<String>,
    #[serde(default, rename = "Type", alias = "journal_type")]
    pub journal_type: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct JournalDetailQuery {
    #[serde(default)]
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct JournalSelectQuery {
    #[serde(flatten)]
    pub filter: JournalListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

async fn load_journal_rows(
    db: &sea_orm::DatabaseConnection,
    q: &JournalListQuery,
    auth: &AuthContext,
) -> ObjectList<JournalRow> {
    let mut query = JournalEntity::find();
    query = apply_journal_filters(
        query,
        q.name.as_deref(),
        q.is_active,
        q.currency_id.as_deref(),
        q.journal_type.as_deref(),
    );
    query = scope_superuser(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(journal::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(journal::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Active DESC") => {
            query.order_by_desc(journal::Column::IsActive)
        }
        s if s.eq_ignore_ascii_case("Active ASC") || s.eq_ignore_ascii_case("Active") => {
            query.order_by_asc(journal::Column::IsActive)
        }
        s if s.eq_ignore_ascii_case("Type DESC") => {
            query.order_by_desc(journal::Column::JournalType)
        }
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(journal::Column::JournalType)
        }
        _ => query.order_by_desc(journal::Column::CreatedAt),
    };
    let page = q.page.get();
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for j in models {
        let currency_label = load_currency_by_id(db, j.currency_id)
            .await
            .map(|c| currency_summary(&c))
            .unwrap_or_else(|| "—".into());
        rows.push(JournalRow {
            id: j.id,
            name: j.name,
            is_active: j.is_active,
            currency_label,
            journal_type: j.journal_type.to_string(),
        });
    }
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

pub async fn list(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<JournalListQuery>,
) -> maud::Markup {
    let journals = load_journal_rows(&state.db, &q, &ctx).await;
    let page = JournalListPage {
        journals,
        filter_name: q.name.clone().unwrap_or_default(),
        filter_is_active: q.is_active.unwrap_or(false),
        filter_currency_id: q.currency_id.clone().unwrap_or_default(),
        filter_journal_type: q.journal_type.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<JournalTableKey>() {
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
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<JournalDetailQuery>,
    Path(id): Path<i64>,
) -> Response {
    let Some(j) = find_journal_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let currency_label = load_currency_by_id(&state.db, j.currency_id)
        .await
        .map(|c| currency_summary(&c))
        .unwrap_or_else(|| "—".into());
    let entries_raw = load_journal_entries_for_journal(&state.db, j.id, q.sort.as_deref()).await;
    let entry_ids: Vec<i64> = entries_raw.iter().map(|e| e.id).collect();
    let amounts = load_journal_entry_transfer_amounts(&state.db, &entry_ids).await;
    let journal_name = j.name.clone();
    let mut entry_rows = Vec::with_capacity(entries_raw.len());
    for e in entries_raw {
        let source_doc = resolve_source_doc_display(&state.db, &source_docs, e.source_doc_id).await;
        entry_rows.push(JournalEntryRow {
            id: e.id,
            datetime: ctx.format_datetime_seconds(e.datetime).into_string(),
            source_doc_label: source_doc.type_label.clone(),
            source_doc_instance_name: source_doc.instance_name,
            source_doc_url: source_doc.detail_url,
            journal_name: journal_name.clone(),
            amount: amounts.get(&e.id).cloned().unwrap_or_else(|| "—".into()),
            label: format!("#{} · {}", e.id, source_doc.type_label),
        });
    }
    let entry_count = entry_rows.len() as u64;
    let entries = ObjectList::from_page(entry_rows, 1, PAGE_SIZE, entry_count);
    let page = JournalDetailPage {
        id: j.id,
        name: j.name,
        is_active: j.is_active,
        is_mutable: j.is_mutable,
        currency_id: j.currency_id,
        currency_label,
        journal_type: j.journal_type.to_string(),
        entries,
        sort: journal_entry_sort(q.sort.as_deref()).to_string(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    if htmx.targets::<JournalTableKey>() {
        return page.render_entries_table().into_response();
    }
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !require_superuser(&ctx) {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let prefs =
        crate::plugins::finance_accounts::preferences::load_accounting_preferences(&state.db).await;
    let (currency_id, currency_display) = match prefs.default_currency_id.filter(|&id| id > 0) {
        Some(id) => {
            let display = load_currency_by_id(&state.db, id)
                .await
                .map(|c| currency_summary(&c))
                .unwrap_or_default();
            (id.to_string(), display)
        }
        None => (String::new(), String::new()),
    };
    let page = JournalCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        is_active: true,
        currency_id,
        currency_display,
        journal_type: "Debit".to_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<JournalCreateForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    }
    let now = Utc::now();
    let jtype = JournalType::parse(&form.journal_type).unwrap_or_default();
    let model = journal::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.clone()),
        is_active: Set(checkbox_on(&form.is_active) || form.is_active.is_empty()),
        is_mutable: Set(false),
        currency_id: Set(parse_i64(&form.currency_id).unwrap_or(0)),
        journal_type: Set(jtype),
        ..Default::default()
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk::<JournalCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &JournalDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let currency_display = if !form.currency_id.is_empty() {
                load_currency_by_id(&state.db, parse_i64(&form.currency_id).unwrap_or(0))
                    .await
                    .map(|c| currency_summary(&c))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let page = JournalCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                is_active: checkbox_on(&form.is_active) || form.is_active.is_empty(),
                currency_id: form.currency_id,
                currency_display,
                journal_type: form.journal_type,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    }
    let Some(j) = find_journal_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let currency_display = load_currency_by_id(&state.db, j.currency_id)
        .await
        .map(|c| currency_summary(&c))
        .unwrap_or_default();
    let page = JournalEditModalPage::from_model(&j, q.form_name(), currency_display);
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<JournalForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    }
    let Some(existing) = find_journal_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let now = Utc::now();
    let jtype = JournalType::parse(&form.journal_type).unwrap_or(existing.journal_type);
    let model = journal::ActiveModel {
        id: Set(existing.id),
        updated_at: Set(Some(now)),
        name: Set(form.name.clone()),
        is_active: Set(checkbox_on(&form.is_active)),
        is_mutable: Set(checkbox_on(&form.is_mutable)),
        currency_id: Set(parse_i64(&form.currency_id).unwrap_or(existing.currency_id)),
        journal_type: Set(jtype),
        ..Default::default()
    };
    match model.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<JournalEditModalKey>(
            &htmx,
            &JournalDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let currency_display = if !form.currency_id.is_empty() {
                load_currency_by_id(&state.db, parse_i64(&form.currency_id).unwrap_or(0))
                    .await
                    .map(|c| currency_summary(&c))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let page = JournalEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                is_active: checkbox_on(&form.is_active),
                is_mutable: checkbox_on(&form.is_mutable),
                currency_id: form.currency_id,
                currency_display,
                journal_type: form.journal_type,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    }
    if find_journal_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    }
    let _ = journal::Entity::delete_by_id(id).exec(&state.db).await;
    Redirect::to(&JournalListRouteTag.url()).into_response()
}

pub async fn select(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<JournalSelectQuery>,
) -> maud::Markup {
    let journals = load_journal_rows(&state.db, &q.filter, &ctx).await;
    let page = JournalSelectPage {
        journals,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        filter_is_active: q.filter.is_active.unwrap_or(false),
        filter_currency_id: q.filter.currency_id.clone().unwrap_or_default(),
        filter_journal_type: q.filter.journal_type.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.unwrap_or_else(|| "JournalID".into()),
    };
    respond_picker_select::<JournalSelectTableKey, JournalSelectModalKey, _>(&htmx, &page)
}
