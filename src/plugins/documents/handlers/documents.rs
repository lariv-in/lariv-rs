use crate::plugins::users::role_authorization::scope_allowed;
use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use crate::{
    components::{ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::users::middleware::RequireAuth,
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};

use crate::plugins::documents::{
    detail_actions::{DocumentDetailActionInput, render_document_actions},
    document_type::DocumentType,
    entities::document::{self, Entity as DocumentEntity},
    forms::DocumentForm,
    handlers::ModalNameQuery,
    keys::{
        DocumentCreateModalKey, DocumentDeleteModalKey, DocumentEditModalKey,
        DocumentSelectModalKey, DocumentSelectTableKey, DocumentTableKey,
    },
    logic::{self, TypeFields},
    routes::{DocumentDefaultRouteTag, DocumentDetailRouteTag},
    scope::{apply_document_sort, apply_name_filter, find_document_scoped},
    state::DocumentsState,
    templates::{
        ConfirmDeletePage, DocumentCreateModalPage, DocumentDetailPage, DocumentEditModalPage,
        DocumentListPage, DocumentOption, DocumentRow, DocumentSelectPage,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct DocumentListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn blank_fields() -> TypeFields {
    TypeFields {
        vnode_id: 0,
        vnode_name: String::new(),
        aadhar_number: String::new(),
        pan_number: String::new(),
        passport_number: String::new(),
        name: String::new(),
        gender: String::new(),
        gender_label: String::new(),
        date_of_birth: String::new(),
        address: String::new(),
        nationality: String::new(),
        expiry_date: String::new(),
    }
}

fn fields_from_form(form: &DocumentForm, vnode_name: String) -> TypeFields {
    let gender_label = crate::plugins::documents::gender::Gender::parse(&form.gender)
        .map(|gender| gender.label().to_string())
        .unwrap_or_else(|| form.gender.clone());
    TypeFields {
        vnode_id: form.vnode_id,
        vnode_name,
        aadhar_number: form.aadhar_number.clone(),
        pan_number: form.pan_number.clone(),
        passport_number: form.passport_number.clone(),
        name: form.name.clone(),
        gender: form.gender.clone(),
        gender_label,
        date_of_birth: form.date_of_birth.clone(),
        address: form.address.clone(),
        nationality: form.nationality.clone(),
        expiry_date: form.expiry_date.clone(),
    }
}

async fn load_document_rows(
    db: &sea_orm::DatabaseConnection,
    q: &DocumentListQuery,
    page_size: u32,
) -> ObjectList<DocumentRow> {
    let mut query = scope_allowed::<super::super::routes::DocumentsView, _>(DocumentEntity::find());

    query = match apply_name_filter(db, query, q.name.as_deref()).await {
        Ok(query) => query,
        Err(err) => {
            tracing::error!(error = %err, "document name filter failed");
            return ObjectList::from_page(Vec::new(), q.page.get(), page_size, 0);
        }
    };
    query = apply_document_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let summaries = logic::load_type_summaries(db, &models)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "load document type rows failed");
            std::collections::HashMap::new()
        });
    let rows = models
        .into_iter()
        .map(|doc| {
            let summary = summaries.get(&doc.id);
            DocumentRow {
                id: doc.id,
                document_type: doc.document_type.label().to_string(),
                name: summary.map(|row| row.name.clone()).unwrap_or_default(),
                number: summary.map(|row| row.number.clone()).unwrap_or_default(),
            }
        })
        .collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<DocumentListQuery>,
) -> maud::Markup {
    let documents = load_document_rows(&state.db, &q, q.page_size.get()).await;
    let page = DocumentListPage {
        documents,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<DocumentTableKey>() {
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
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(doc) = find_document_scoped(&state.db, id).await else {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    };
    let (fields, error) = match logic::load_type_fields(&state.db, &doc).await {
        Ok(fields) => (fields, String::new()),
        Err(err) => (blank_fields(), err),
    };
    let extra_actions = render_document_actions(&DocumentDetailActionInput {
        db: &state.db,
        auth: &ctx,
        document_id: doc.id,
        vnode_name: &fields.vnode_name,
    })
    .await;
    let page = detail_page(doc.id, doc.document_type, fields, error, extra_actions);
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn detail_page(
    id: i64,
    document_type: DocumentType,
    fields: TypeFields,
    error: String,
    extra_actions: String,
) -> DocumentDetailPage {
    DocumentDetailPage {
        id,
        document_type: document_type.label().to_string(),
        name: fields.name,
        vnode_id: fields.vnode_id,
        vnode_name: fields.vnode_name,
        aadhar_number: fields.aadhar_number,
        pan_number: fields.pan_number,
        passport_number: fields.passport_number,
        gender: fields.gender_label,
        date_of_birth: fields.date_of_birth,
        address: fields.address,
        nationality: fields.nationality,
        expiry_date: fields.expiry_date,
        error,
        extra_actions,
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct DocumentCreateQuery {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    refresh: Option<String>,
    #[serde(default)]
    target_input: Option<String>,
    #[serde(default)]
    document_type: Option<String>,
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<DocumentCreateQuery>,
) -> maud::Markup {
    let document_type = q
        .document_type
        .as_deref()
        .and_then(DocumentType::parse)
        .unwrap_or_default()
        .as_str()
        .to_string();
    let page = DocumentCreateModalPage {
        form_name: q.name.unwrap_or_default(),
        refresh_table: q.refresh.unwrap_or_default(),
        target_input: q.target_input.unwrap_or_default(),
        document_type,
        fields: blank_fields(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<DocumentForm>,
) -> Response {
    match logic::create_document(&state.db, &form).await {
        Ok(saved) => {
            let name = form.name.trim();
            let label = if name.is_empty() {
                saved.document_type.label()
            } else {
                name
            };
            respond_create_modal_done_fk::<DocumentCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &DocumentDetailRouteTag::new(saved.id).url(),
                saved.id,
                label,
                &q.target_input(),
            )
        }
        Err(err) => {
            let vnode_name = vnode_label(&state.db, form.vnode_id).await;
            let page = DocumentCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                document_type: form.document_type.clone(),
                fields: fields_from_form(&form, vnode_name),
                error: err,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(doc) = find_document_scoped(&state.db, id).await else {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    };
    let fields = match logic::load_type_fields(&state.db, &doc).await {
        Ok(fields) => fields,
        Err(err) => {
            let page = DocumentEditModalPage {
                id: doc.id,
                form_name: q.form_name(),
                document_type: doc.document_type.as_str().to_string(),
                fields: blank_fields(),
                error: err,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let page = DocumentEditModalPage {
        id: doc.id,
        form_name: q.form_name(),
        document_type: doc.document_type.as_str().to_string(),
        fields,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<DocumentForm>,
) -> Response {
    let Some(existing) = find_document_scoped(&state.db, id).await else {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    };
    match logic::update_document(&state.db, &existing, &form).await {
        Ok(()) => respond_edit_modal_done::<DocumentEditModalKey>(
            &htmx,
            &DocumentDetailRouteTag::new(id).url(),
        ),
        Err(err) => {
            let vnode_name = vnode_label(&state.db, form.vnode_id).await;
            let page = DocumentEditModalPage {
                id,
                form_name: q.form_name(),
                document_type: form.document_type.clone(),
                fields: fields_from_form(&form, vnode_name),
                error: err,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: DocumentDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this document?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_documents.DocumentDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<DocumentsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(existing) = find_document_scoped(&state.db, id).await else {
        return Redirect::to(&DocumentDefaultRouteTag.url()).into_response();
    };
    match logic::delete_document(&state.db, &existing).await {
        Ok(()) => htmx.redirect(&DocumentDefaultRouteTag.url()),
        Err(err) => {
            tracing::error!(error = %err, id, "failed to delete document");
            let page = ConfirmDeletePage {
                modal_uid: DocumentDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this document?".into(),
                form_name: "p_documents.DocumentDeleteForm".into(),
                id,
                error: err,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select_aadhar(
    state: Cap<DocumentsState>,
    auth: RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    select_documents(state, auth, htmx, uri, DocumentType::AadharCard).await
}

pub async fn select_pan(
    state: Cap<DocumentsState>,
    auth: RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    select_documents(state, auth, htmx, uri, DocumentType::Pan).await
}

pub async fn select_passport(
    state: Cap<DocumentsState>,
    auth: RequireAuth,
    htmx: Htmx,
    uri: Uri,
) -> maud::Markup {
    select_documents(state, auth, htmx, uri, DocumentType::Passport).await
}

async fn select_documents(
    Cap(state): Cap<DocumentsState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    document_type: DocumentType,
) -> maud::Markup {
    let q = DocumentListQuery::from_uri(&uri);
    let page = q.page.get();
    let page_size = q.page_size.get();
    let mut query = scope_allowed::<super::super::routes::DocumentsView, _>(DocumentEntity::find())
        .filter(document::Column::DocumentType.eq(document_type));
    query = match apply_name_filter(&state.db, query, q.name.as_deref()).await {
        Ok(query) => query,
        Err(err) => {
            tracing::error!(error = %err, "document picker filter failed");
            DocumentEntity::find().filter(document::Column::Id.eq(0))
        }
    };
    query = query.order_by_desc(document::Column::Id);
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let summaries = logic::load_type_summaries(&state.db, &models)
        .await
        .unwrap_or_default();
    let rows = models
        .into_iter()
        .map(|doc| {
            let summary = summaries.get(&doc.id);
            let name = summary.map(|row| row.name.clone()).unwrap_or_default();
            let number = summary.map(|row| row.number.clone()).unwrap_or_default();
            let label = if number.is_empty() {
                name.clone()
            } else if name.is_empty() {
                number.clone()
            } else {
                format!("{name} · {number}")
            };
            DocumentOption { id: doc.id, label }
        })
        .collect();
    let page = DocumentSelectPage {
        documents: ObjectList::from_page(rows, page, page_size, total),
        filter_name: q.name.unwrap_or_default(),
        target_input: q.target_input.clone().unwrap_or_else(|| "document".into()),
        path_and_query: path_and_query(&uri),
        page_size,
        title: document_type.label().to_string(),
    };
    respond_picker_select::<DocumentSelectTableKey, DocumentSelectModalKey, _>(&htmx, &page)
}

impl DocumentListQuery {
    fn from_uri(uri: &Uri) -> Self {
        let Some(query) = uri.query() else {
            return Self::default();
        };
        crate::html_form::UrlencodedFields::parse(query.as_bytes())
            .ok()
            .and_then(|fields| fields.deserialize().ok())
            .unwrap_or_default()
    }
}

async fn vnode_label(db: &sea_orm::DatabaseConnection, vnode_id: i64) -> String {
    if vnode_id <= 0 {
        return String::new();
    }
    crate::web::opt_or_log(
        crate::plugins::filesystem::entities::filesystem_node::Entity::find_by_id(vnode_id)
            .one(db)
            .await,
        "find document file label",
    )
    .map(|node| node.name)
    .unwrap_or_default()
}
