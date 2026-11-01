use std::collections::HashMap;

use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use maud::{Markup, html};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;
use lariv_plugin_users::role_authorization::scope_allowed;

use crate::{
    entities::{
        account::Entity as AccountEntity,
        custom_source_doc::{self, Entity as CustomSourceDocEntity},
        journal::{self, Entity as JournalEntity},
        journal_entry::{self, Entity as JournalEntryEntity},
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
    },
    forms::{CustomSourceDocForm, CustomSourceDocIndexCreateForm},
    journal_lines_editor::input_journal_lines,
    journal_template,
    keys::CustomSourceDocIndexCreateModalKey,
    logic::{
        custom_source_doc::{
            create_custom_source_doc, delete_custom_source_doc, empty_lines_json,
            ensure_posting_accounts, journal_specs_editor_json, lines_form_value,
            parse_journal_lines, update_custom_source_doc,
        },
        journal::{JournalLineSpec, cascade_delete_preview, collect_journal_entry_cascade},
    },
    routes::{
        CustomSourceDocDetailRouteTag, CustomSourceDocEditPostRouteTag,
        CustomSourceDocIndexRouteTag,
    },
    scope::{
        find_custom_source_doc_template_scoped, find_journal_scoped, load_default_journal,
        load_journal_display_label,
    },
    source_doc_registry::SourceDocRegistry,
    state::AccountsState,
    templates::{
        CustomSourceDocDeleteItem, CustomSourceDocDeleteModalPage, CustomSourceDocDetailPage,
        CustomSourceDocFormPage, CustomSourceDocIndexCreateModalPage, CustomSourceDocIndexPage,
        CustomSourceDocIndexRow, CustomSourceDocLineRow,
    },
    variable_values_editor::{
        empty_variable_values_fields, input_variable_values_fields, values_json_from_form_map,
    },
};

use super::ModalNameQuery;
use super::util::{
    optional_fk, optional_fk_form_value, parse_i64, path_and_query, vnode_display, vnode_link,
};

#[derive(Debug, Deserialize, Default)]
pub struct CustomSourceDocIndexQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

fn can_mutate() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        crate::routes::FinanceAccountsMutate,
    >())
}

pub async fn index(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CustomSourceDocIndexQuery>,
) -> maud::Markup {
    let (docs, total, prepared) = fetch_index_docs(
        &state.db,
        uri.query().unwrap_or(""),
        q.sort.as_deref(),
        q.page.get(),
        q.page_size.get(),
    )
    .await;
    let entry_ids: Vec<i64> = docs.iter().map(|doc| doc.journal_entry_id).collect();
    let journals = journal_refs_by_entry(&state.db, &entry_ids).await;
    let mut rows = Vec::with_capacity(docs.len());
    for doc in docs {
        let (journal_id, journal_name) = journals
            .get(&doc.journal_entry_id)
            .cloned()
            .unwrap_or_else(|| (0, "—".into()));
        rows.push(CustomSourceDocIndexRow {
            id: doc.id,
            title: doc.title,
            journal_id,
            journal_name,
            created_by: user_name(&state.db, doc.created_by_id).await,
            created_at: doc
                .created_at
                .map(|ts| ctx.format_datetime_seconds(ts).into_string())
                .unwrap_or_else(|| "—".into()),
            journal_entry_id: doc.journal_entry_id,
        });
    }
    let page = CustomSourceDocIndexPage {
        docs: ObjectList::from_page(rows, q.page.get(), q.page_size.get(), total),
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_create: can_mutate(),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<crate::keys::CustomSourceDocIndexTableKey>() {
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

#[derive(Debug, Deserialize, Default)]
pub struct IndexCreateGetQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default)]
    pub template_id: Option<String>,
}

pub async fn index_create_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<IndexCreateGetQuery>,
) -> maud::Markup {
    let (journal_id, journal_display) = match load_default_journal(&state.db).await {
        Some(journal) if journal.is_mutable => (journal.id.to_string(), journal.name),
        _ => (String::new(), String::new()),
    };
    let (template_id, template_display, variables_schema, title, content, attached_vnode_id) =
        match q.template_id.as_deref().and_then(parse_i64).filter(|id| *id > 0) {
            Some(id) => match find_custom_source_doc_template_scoped(&state.db, id).await {
                Some(t) => (
                    t.id.to_string(),
                    t.name.clone(),
                    t.variables.clone(),
                    t.name,
                    t.remarks.unwrap_or_default(),
                    t.attached_vnode_id,
                ),
                None => (
                    String::new(),
                    String::new(),
                    "{}".into(),
                    String::new(),
                    String::new(),
                    None,
                ),
            },
            None => (
                String::new(),
                String::new(),
                "{}".into(),
                String::new(),
                String::new(),
                None,
            ),
        };
    let attached_vnode_display = vnode_display(&state.db, attached_vnode_id).await;
    let page = index_form_page(IndexFormDraft {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        journal_id,
        journal_display,
        template_id,
        template_display,
        variables_schema,
        variable_values: "{}".into(),
        title,
        content,
        attached_vnode_id: optional_fk_form_value(attached_vnode_id),
        attached_vnode_display,
        lines: empty_lines_json(),
        error: String::new(),
    });
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

#[derive(Debug, Deserialize, Default)]
pub struct TemplateVariablesQuery {
    #[serde(default)]
    pub template_id: Option<String>,
}

pub async fn template_variables(
    Cap(state): Cap<AccountsState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<TemplateVariablesQuery>,
) -> Markup {
    let Some(id) = q
        .template_id
        .as_deref()
        .and_then(parse_i64)
        .filter(|id| *id > 0)
    else {
        return empty_variable_values_fields();
    };
    let Some(template) = find_custom_source_doc_template_scoped(&state.db, id).await else {
        return empty_variable_values_fields();
    };
    input_variable_values_fields(&template.variables, "{}")
}

pub async fn fill_journal_lines(
    Cap(state): Cap<AccountsState>,
    RequireAuth(_ctx): RequireAuth,
    Form(map): Form<HashMap<String, String>>,
) -> Markup {
    let lines_field = map.get("Lines").cloned().unwrap_or_else(empty_lines_json);
    let render_err = |error: String| {
        html! {
            p class="text-error mb-2" { (error) }
            (input_journal_lines("Lines", "Lines", &lines_field))
        }
    };
    let Some(template_id) = map
        .get("TemplateID")
        .and_then(|s| parse_i64(s))
        .filter(|id| *id > 0)
    else {
        return render_err("Select a transaction template first".into());
    };
    let Some(template) = find_custom_source_doc_template_scoped(&state.db, template_id).await else {
        return render_err("Transaction template not found".into());
    };
    let variable_values = values_json_from_form_map(&template.variables, &map);
    let items = match journal_template::load_template_lines(&state.db, template.id).await {
        Ok(items) => items,
        Err(e) => return render_err(e),
    };
    let parsed = journal_template::parsed_lines_from_items(&items);
    let evaluated =
        match journal_template::evaluate_template_lines(&template.variables, &parsed, &variable_values)
        {
            Ok(lines) => lines,
            Err(e) => return render_err(e),
        };
    let specs: Vec<JournalLineSpec> = evaluated
        .into_iter()
        .map(|line| JournalLineSpec {
            account_id: line.account_id,
            amount: line.amount,
        })
        .collect();
    match journal_specs_editor_json(&state.db, &specs, &[], "journal-line").await {
        Ok(lines) => input_journal_lines("Lines", "Lines", &lines),
        Err(e) => render_err(e),
    }
}

pub async fn index_create_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CustomSourceDocIndexCreateForm>,
) -> Response {
    let journal_id_raw = form.journal_id.clone();
    let template_id_raw = form.template_id.clone();
    let title = form.title.clone();
    let content = form.content.clone();
    let attached_vnode_id_raw = form.attached_vnode_id.clone();
    let variable_values = if form.variable_values.trim().is_empty() {
        "{}".into()
    } else {
        form.variable_values.clone()
    };
    let lines_raw = submitted_lines(&form.lines);
    let form_name = q.form_name();
    let refresh_table = q.refresh_table();
    let journal_display = journal_picker_label(&state.db, &journal_id_raw).await;
    let (template_display, variables_schema) =
        template_picker_state(&state.db, &template_id_raw).await;
    let attached_vnode_display = vnode_display(&state.db, optional_fk(&attached_vnode_id_raw)).await;
    let render_err = |error: String| {
        let page = index_form_page(IndexFormDraft {
            form_name: form_name.clone(),
            refresh_table: refresh_table.clone(),
            journal_id: journal_id_raw.clone(),
            journal_display: journal_display.clone(),
            template_id: template_id_raw.clone(),
            template_display: template_display.clone(),
            variables_schema: variables_schema.clone(),
            variable_values: variable_values.clone(),
            title: title.clone(),
            content: content.clone(),
            attached_vnode_id: attached_vnode_id_raw.clone(),
            attached_vnode_display: attached_vnode_display.clone(),
            lines: lines_raw.clone(),
            error,
        });
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };
    let Some(journal_id) = parse_i64(&journal_id_raw).filter(|id| *id > 0) else {
        return render_err("Journal is required".into());
    };
    if find_journal_scoped(&state.db, journal_id).await.is_none() {
        return render_err("Journal not found".into());
    }
    let lines = match parse_journal_lines(&form.lines) {
        Ok(lines) => lines,
        Err(e) => return render_err(e),
    };
    if let Err(e) = ensure_posting_accounts(&state.db, &lines).await {
        return render_err(e);
    }
    match create_custom_source_doc(
        &state.db,
        journal_id,
        &form.title,
        &form.content,
        optional_fk(&form.attached_vnode_id),
        ctx.user.id,
        &lines,
    )
    .await
    {
        Ok(model) => respond_create_modal_done::<CustomSourceDocIndexCreateModalKey>(
            &htmx,
            &refresh_table,
            &CustomSourceDocDetailRouteTag::new(model.id).url(),
        ),
        Err(e) => render_err(e),
    }
}

pub async fn detail(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(loaded) = load_doc(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocIndexRouteTag.url()).into_response();
    };
    let lines = load_line_rows(&state.db, loaded.doc.journal_entry_id).await;
    let (attached_vnode_href, attached_vnode_label) =
        vnode_link(&state.db, loaded.doc.attached_vnode_id).await;
    let page = CustomSourceDocDetailPage {
        id: loaded.doc.id,
        title: loaded.doc.title,
        content: loaded.doc.content,
        attached_vnode_href,
        attached_vnode_label,
        created_by: user_name(&state.db, loaded.doc.created_by_id).await,
        created_at: loaded
            .doc
            .created_at
            .map(|ts| ctx.format_datetime_seconds(ts).into_string())
            .unwrap_or_else(|| "—".into()),
        journal_entry_id: loaded.doc.journal_entry_id,
        lines,
        can_mutate: can_mutate() && loaded.journal.is_mutable,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(loaded) = load_doc(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocIndexRouteTag.url()).into_response();
    };
    let lines = lines_form_value(&state.db, loaded.doc.journal_entry_id)
        .await
        .unwrap_or_else(|_err| empty_lines_json());
    let mutable = can_mutate() && loaded.journal.is_mutable;
    let attached_vnode_display = vnode_display(&state.db, loaded.doc.attached_vnode_id).await;
    let page = form_page(
        id,
        loaded.journal.name,
        FormDraft {
            heading: "Edit transaction",
            post_path: CustomSourceDocEditPostRouteTag::new(id).path(),
            title: loaded.doc.title,
            content: loaded.doc.content,
            attached_vnode_id: optional_fk_form_value(loaded.doc.attached_vnode_id),
            attached_vnode_display,
            lines,
            error: if mutable {
                String::new()
            } else {
                "This journal is immutable. Enable Mutable on the journal edit page before editing source documents.".into()
            },
            can_save: mutable,
        },
    );
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<CustomSourceDocForm>,
) -> Response {
    let Some(loaded) = load_doc(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocIndexRouteTag.url()).into_response();
    };
    let journal_name = loaded.journal.name.clone();
    let mutable = can_mutate() && loaded.journal.is_mutable;
    let attached_vnode_display =
        vnode_display(&state.db, optional_fk(&form.attached_vnode_id)).await;
    let render_err = |error: String| {
        let page = form_page(
            id,
            journal_name.clone(),
            FormDraft {
                heading: "Edit transaction",
                post_path: CustomSourceDocEditPostRouteTag::new(id).path(),
                title: form.title.clone(),
                content: form.content.clone(),
                attached_vnode_id: form.attached_vnode_id.clone(),
                attached_vnode_display: attached_vnode_display.clone(),
                lines: submitted_lines(&form.lines),
                error,
                can_save: mutable,
            },
        );
        html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response()
    };
    let lines = match parse_journal_lines(&form.lines) {
        Ok(lines) => lines,
        Err(e) => return render_err(e),
    };
    if let Err(e) = ensure_posting_accounts(&state.db, &lines).await {
        return render_err(e);
    }
    match update_custom_source_doc(
        &state.db,
        id,
        &form.title,
        &form.content,
        optional_fk(&form.attached_vnode_id),
        &lines,
    )
    .await
    {
        Ok(()) => Redirect::to(&CustomSourceDocDetailRouteTag::new(id).url()).into_response(),
        Err(e) => render_err(e),
    }
}

pub async fn delete_get(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(loaded) = load_doc(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocIndexRouteTag.url()).into_response();
    };
    let page = build_delete_modal(&state.db, &source_docs, &loaded, None).await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn delete_post(
    Cap(state): Cap<AccountsState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(loaded) = load_doc(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocIndexRouteTag.url()).into_response();
    };
    if !loaded.journal.is_mutable || !can_mutate() {
        let page = build_delete_modal(&state.db, &source_docs, &loaded, None).await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    match delete_custom_source_doc(&state.db, id).await {
        Ok(()) => htmx.redirect(&CustomSourceDocIndexRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete custom source doc");
            let page = build_delete_modal(&state.db, &source_docs, &loaded, Some(e)).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

struct LoadedDoc {
    doc: custom_source_doc::Model,
    journal: crate::entities::journal::Model,
}

async fn load_doc(db: &sea_orm::DatabaseConnection, id: i64) -> Option<LoadedDoc> {
    let doc = CustomSourceDocEntity::find_by_id(id).one(db).await.ok()??;
    let entry = JournalEntryEntity::find_by_id(doc.journal_entry_id)
        .one(db)
        .await
        .ok()??;
    let journal = find_journal_scoped(db, entry.journal_id).await?;
    Some(LoadedDoc { doc, journal })
}

async fn load_line_rows(
    db: &sea_orm::DatabaseConnection,
    journal_entry_id: i64,
) -> Vec<CustomSourceDocLineRow> {
    let items = JournalEntryItemEntity::find()
        .filter(journal_entry_item::Column::JournalEntryId.eq(journal_entry_id))
        .order_by_asc(journal_entry_item::Column::Id)
        .all(db)
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        let acct = AccountEntity::find_by_id(item.account_id)
            .one(db)
            .await
            .ok()
            .flatten();
        let account_label = acct
            .as_ref()
            .map(|a| format!("{} — {}", a.code, a.name))
            .unwrap_or_else(|| format!("Account #{}", item.account_id));
        let (debit, credit) = if item.amount.is_sign_negative() {
            (String::new(), decimal_text(item.amount.abs()))
        } else if item.amount.is_zero() {
            (String::new(), String::new())
        } else {
            (decimal_text(item.amount), String::new())
        };
        rows.push(CustomSourceDocLineRow {
            account_id: item.account_id,
            account_label,
            debit,
            credit,
        });
    }
    rows
}

fn decimal_text(amount: rust_decimal::Decimal) -> String {
    lariv_plugin_finance_common::decimal::decimal_display(amount)
}

async fn user_name(db: &sea_orm::DatabaseConnection, user_id: i64) -> String {
    let name = lariv_plugin_users::entities::UserEntity::find_by_id(user_id)
        .one(db)
        .await
        .ok()
        .flatten()
        .map(|user| user.name);
    match name {
        Some(name) if !name.trim().is_empty() => name,
        _ => format!("User #{user_id}"),
    }
}

struct FormDraft {
    heading: &'static str,
    post_path: String,
    title: String,
    content: String,
    attached_vnode_id: String,
    attached_vnode_display: String,
    lines: String,
    error: String,
    can_save: bool,
}

fn submitted_lines(raw: &str) -> String {
    if raw.trim().is_empty() {
        empty_lines_json()
    } else {
        raw.to_string()
    }
}

async fn fetch_index_docs(
    db: &sea_orm::DatabaseConnection,
    query_string: &str,
    sort: Option<&str>,
    page: u32,
    page_size: u32,
) -> (
    Vec<custom_source_doc::Model>,
    u64,
    crate::filters::PreparedFilters,
) {
    let prepared = crate::filters::prepare(
        query_string,
        crate::filters::custom_source_doc_fields(),
        db.get_database_backend(),
    );
    let mut query = scope_allowed::<super::super::routes::FinanceAccountsView, _>(
        CustomSourceDocEntity::find(),
    );
    query = query.filter(prepared.condition.clone());
    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Title DESC") => {
            query.order_by_desc(custom_source_doc::Column::Title)
        }
        s if s.eq_ignore_ascii_case("Title ASC") || s.eq_ignore_ascii_case("Title") => {
            query.order_by_asc(custom_source_doc::Column::Title)
        }
        s if s.eq_ignore_ascii_case("CreatedAt ASC") || s.eq_ignore_ascii_case("CreatedAt") => {
            query.order_by_asc(custom_source_doc::Column::CreatedAt)
        }
        _ => query
            .order_by_desc(custom_source_doc::Column::CreatedAt)
            .order_by_desc(custom_source_doc::Column::Id),
    };
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let docs = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (docs, total, prepared)
}

async fn journal_refs_by_entry(
    db: &sea_orm::DatabaseConnection,
    entry_ids: &[i64],
) -> HashMap<i64, (i64, String)> {
    if entry_ids.is_empty() {
        return HashMap::new();
    }
    let entries = JournalEntryEntity::find()
        .filter(journal_entry::Column::Id.is_in(entry_ids.to_vec()))
        .all(db)
        .await
        .unwrap_or_default();
    let journal_ids: Vec<i64> = entries.iter().map(|entry| entry.journal_id).collect();
    if journal_ids.is_empty() {
        return HashMap::new();
    }
    let journals = JournalEntity::find()
        .filter(journal::Column::Id.is_in(journal_ids))
        .all(db)
        .await
        .unwrap_or_default();
    let names: HashMap<i64, String> = journals
        .into_iter()
        .map(|journal| (journal.id, journal.name))
        .collect();
    entries
        .into_iter()
        .filter_map(|entry| {
            let name = names.get(&entry.journal_id)?.clone();
            Some((entry.id, (entry.journal_id, name)))
        })
        .collect()
}

async fn journal_picker_label(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    match parse_i64(raw).filter(|id| *id > 0) {
        Some(id) => load_journal_display_label(db, Some(id)).await,
        None => String::new(),
    }
}

struct IndexFormDraft {
    form_name: String,
    refresh_table: String,
    journal_id: String,
    journal_display: String,
    template_id: String,
    template_display: String,
    variables_schema: String,
    variable_values: String,
    title: String,
    content: String,
    attached_vnode_id: String,
    attached_vnode_display: String,
    lines: String,
    error: String,
}

fn index_form_page(draft: IndexFormDraft) -> CustomSourceDocIndexCreateModalPage {
    CustomSourceDocIndexCreateModalPage {
        form_name: draft.form_name,
        refresh_table: draft.refresh_table,
        journal_id: draft.journal_id,
        journal_display: draft.journal_display,
        template_id: draft.template_id,
        template_display: draft.template_display,
        variables_schema: draft.variables_schema,
        variable_values: draft.variable_values,
        title: draft.title,
        content: draft.content,
        attached_vnode_id: draft.attached_vnode_id,
        attached_vnode_display: draft.attached_vnode_display,
        lines: draft.lines,
        error: draft.error,
    }
}

async fn template_picker_state(
    db: &sea_orm::DatabaseConnection,
    raw: &str,
) -> (String, String) {
    match parse_i64(raw).filter(|id| *id > 0) {
        Some(id) => match find_custom_source_doc_template_scoped(db, id).await {
            Some(t) => (t.name, t.variables),
            None => (String::new(), "{}".into()),
        },
        None => (String::new(), "{}".into()),
    }
}

fn form_page(id: i64, journal_name: String, draft: FormDraft) -> CustomSourceDocFormPage {
    CustomSourceDocFormPage {
        id,
        journal_name,
        heading: draft.heading.to_string(),
        post_path: draft.post_path,
        title: draft.title,
        content: draft.content,
        attached_vnode_id: draft.attached_vnode_id,
        attached_vnode_display: draft.attached_vnode_display,
        lines: draft.lines,
        error: draft.error,
        can_save: draft.can_save,
    }
}

async fn build_delete_modal(
    db: &sea_orm::DatabaseConnection,
    registry: &SourceDocRegistry,
    loaded: &LoadedDoc,
    error: Option<String>,
) -> CustomSourceDocDeleteModalPage {
    if !loaded.journal.is_mutable {
        return CustomSourceDocDeleteModalPage {
            id: loaded.doc.id,
            items: Vec::new(),
            can_delete: false,
            error: Some(
                "This journal is immutable. Enable Mutable on the journal edit page before deleting source documents."
                    .into(),
            ),
        };
    }
    let (items, collect_error) =
        match collect_journal_entry_cascade(db, loaded.doc.journal_entry_id).await {
            Ok(graph) => match cascade_delete_preview(db, registry, &graph).await {
                Ok(items) => (
                    items
                        .into_iter()
                        .map(|item| CustomSourceDocDeleteItem {
                            kind: item.kind,
                            label: item.label,
                            url: item.url,
                        })
                        .collect(),
                    None,
                ),
                Err(e) => (Vec::new(), Some(e.to_string())),
            },
            Err(e) => (Vec::new(), Some(e.to_string())),
        };
    CustomSourceDocDeleteModalPage {
        id: loaded.doc.id,
        items,
        can_delete: collect_error.is_none(),
        error: collect_error.or(error),
    }
}
