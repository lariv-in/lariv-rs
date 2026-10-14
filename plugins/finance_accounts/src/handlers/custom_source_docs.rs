use axum::{
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::{
    entities::{
        account::Entity as AccountEntity,
        custom_source_doc::{self, Entity as CustomSourceDocEntity},
        journal_entry::Entity as JournalEntryEntity,
        journal_entry_item::{self, Entity as JournalEntryItemEntity},
    },
    forms::CustomSourceDocForm,
    logic::{
        custom_source_doc::{
            create_custom_source_doc, delete_custom_source_doc, empty_lines_json,
            ensure_posting_accounts, lines_form_value, list_for_journal, parse_journal_lines,
            update_custom_source_doc,
        },
        journal::{cascade_delete_preview, collect_journal_entry_cascade},
    },
    routes::{
        CustomSourceDocCreatePostRouteTag, CustomSourceDocDetailRouteTag,
        CustomSourceDocEditPostRouteTag, JournalCustomSourceDocsRouteTag, JournalListRouteTag,
    },
    scope::find_journal_scoped,
    source_doc_registry::SourceDocRegistry,
    state::AccountsState,
    templates::{
        CustomSourceDocDeleteItem, CustomSourceDocDeleteModalPage, CustomSourceDocDetailPage,
        CustomSourceDocFormPage, CustomSourceDocLineRow, CustomSourceDocListPage,
        CustomSourceDocRow,
    },
};

fn can_mutate() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        crate::routes::FinanceAccountsMutate,
    >())
}

pub async fn list(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(journal_id): Path<i64>,
) -> Response {
    let Some(journal) = find_journal_scoped(&state.db, journal_id).await else {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let docs = list_for_journal(&state.db, journal.id)
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(docs.len());
    for doc in docs {
        rows.push(CustomSourceDocRow {
            id: doc.id,
            title: doc.title,
            created_by: user_name(&state.db, doc.created_by_id).await,
            created_at: doc
                .created_at
                .map(|ts| ctx.format_datetime_seconds(ts).into_string())
                .unwrap_or_else(|| "—".into()),
            journal_entry_id: doc.journal_entry_id,
        });
    }
    let page = CustomSourceDocListPage {
        journal_id: journal.id,
        journal_name: journal.name,
        rows,
        can_create: can_mutate() && journal.is_mutable,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(journal_id): Path<i64>,
) -> Response {
    let Some(journal) = find_journal_scoped(&state.db, journal_id).await else {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let mutable = can_mutate() && journal.is_mutable;
    let page = form_page(
        journal.id,
        journal.name,
        FormDraft {
            heading: "New custom source doc",
            post_path: CustomSourceDocCreatePostRouteTag::new(journal_id).path(),
            title: String::new(),
            content: String::new(),
            lines: empty_lines_json(),
            error: if mutable {
                String::new()
            } else {
                "This journal is immutable. Enable Mutable on the journal edit page before adding source documents.".into()
            },
            can_save: mutable,
        },
    );
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(journal_id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<CustomSourceDocForm>,
) -> Response {
    let Some(journal) = find_journal_scoped(&state.db, journal_id).await else {
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let render_err = |error: String| {
        let page = form_page(
            journal.id,
            journal.name.clone(),
            FormDraft {
                heading: "New custom source doc",
                post_path: CustomSourceDocCreatePostRouteTag::new(journal_id).path(),
                title: form.title.clone(),
                content: form.content.clone(),
                lines: submitted_lines(&form.lines),
                error,
                can_save: can_mutate() && journal.is_mutable,
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
    match create_custom_source_doc(
        &state.db,
        journal.id,
        &form.title,
        &form.content,
        ctx.user.id,
        &lines,
    )
    .await
    {
        Ok(model) => {
            Redirect::to(&CustomSourceDocDetailRouteTag::new(model.id).url()).into_response()
        }
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
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let lines = load_line_rows(&state.db, loaded.doc.journal_entry_id).await;
    let page = CustomSourceDocDetailPage {
        id: loaded.doc.id,
        journal_id: loaded.journal.id,
        journal_name: loaded.journal.name,
        title: loaded.doc.title,
        content: loaded.doc.content,
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
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let lines = lines_form_value(&state.db, loaded.doc.journal_entry_id)
        .await
        .unwrap_or_else(|_err| empty_lines_json());
    let mutable = can_mutate() && loaded.journal.is_mutable;
    let page = form_page(
        loaded.journal.id,
        loaded.journal.name,
        FormDraft {
            heading: "Edit custom source doc",
            post_path: CustomSourceDocEditPostRouteTag::new(id).path(),
            title: loaded.doc.title,
            content: loaded.doc.content,
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
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let journal_id = loaded.journal.id;
    let journal_name = loaded.journal.name.clone();
    let mutable = can_mutate() && loaded.journal.is_mutable;
    let render_err = |error: String| {
        let page = form_page(
            journal_id,
            journal_name.clone(),
            FormDraft {
                heading: "Edit custom source doc",
                post_path: CustomSourceDocEditPostRouteTag::new(id).path(),
                title: form.title.clone(),
                content: form.content.clone(),
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
    match update_custom_source_doc(&state.db, id, &form.title, &form.content, &lines).await {
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
        return Redirect::to(&JournalListRouteTag.url()).into_response();
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
        return Redirect::to(&JournalListRouteTag.url()).into_response();
    };
    let journal_id = loaded.journal.id;
    if !loaded.journal.is_mutable || !can_mutate() {
        let page = build_delete_modal(&state.db, &source_docs, &loaded, None).await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    match delete_custom_source_doc(&state.db, id).await {
        Ok(()) => htmx.redirect(&JournalCustomSourceDocsRouteTag::new(journal_id).url()),
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

fn form_page(journal_id: i64, journal_name: String, draft: FormDraft) -> CustomSourceDocFormPage {
    CustomSourceDocFormPage {
        journal_id,
        journal_name,
        heading: draft.heading.to_string(),
        post_path: draft.post_path,
        title: draft.title,
        content: draft.content,
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
