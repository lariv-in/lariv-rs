use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;

use lariv_core::components::picker::respond_picker_select;
use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_formula::variables;
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::custom_source_doc_template::{self, Entity as TemplateEntity},
    forms::CustomSourceDocTemplateForm,
    handlers::ModalNameQuery,
    journal_template,
    keys::{
        CUSTOM_SOURCE_DOC_TEMPLATE_CREATE_MODAL, CUSTOM_SOURCE_DOC_TEMPLATE_DELETE_MODAL,
        CUSTOM_SOURCE_DOC_TEMPLATE_EDIT_MODAL, CUSTOM_SOURCE_DOC_TEMPLATE_SELECT_MODAL,
        CUSTOM_SOURCE_DOC_TEMPLATE_SELECT_TABLE, CUSTOM_SOURCE_DOC_TEMPLATE_TABLE,
    },
    routes::{CustomSourceDocTemplateDetailRouteTag, CustomSourceDocTemplateListRouteTag},
    scope::find_custom_source_doc_template_scoped,
    state::AccountsState,
    templates::{
        ConfirmDeletePage, CustomSourceDocTemplateCreateModalPage,
        CustomSourceDocTemplateDetailPage, CustomSourceDocTemplateEditModalPage,
        CustomSourceDocTemplateListPage, CustomSourceDocTemplateRow,
        CustomSourceDocTemplateSelectPage,
    },
};

use super::util::{optional_fk, optional_fk_form_value, path_and_query, vnode_display, vnode_link};

#[derive(Debug, Deserialize, Default)]
pub struct TemplateListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

async fn load_template_rows(
    db: &sea_orm::DatabaseConnection,
    query_string: &str,
    sort: Option<&str>,
    page: u32,
    page_size: u32,
) -> (
    ObjectList<CustomSourceDocTemplateRow>,
    crate::filters::PreparedFilters,
) {
    let prepared = crate::filters::prepare(
        query_string,
        crate::filters::custom_source_doc_template_fields(),
        db.get_database_backend(),
    );
    let mut query =
        scope_allowed::<super::super::routes::FinanceAccountsView, _>(TemplateEntity::find());
    query = query.filter(prepared.condition.clone());

    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => {
            query.order_by_desc(custom_source_doc_template::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(custom_source_doc_template::Column::Name)
        }
        _ => query.order_by_desc(custom_source_doc_template::Column::Id),
    };
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows: Vec<CustomSourceDocTemplateRow> = models
        .into_iter()
        .map(|t| CustomSourceDocTemplateRow {
            id: t.id,
            name: t.name,
            remarks: t.remarks.unwrap_or_default(),
            variables: variables::schema_entries_from_stored(&t.variables).join(", "),
        })
        .collect();
    (
        ObjectList::from_page(rows, page, page_size, total),
        prepared,
    )
}

fn optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn can_mutate() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        crate::routes::FinanceAccountsMutate,
    >())
}

async fn save_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &CustomSourceDocTemplateForm,
    id: Option<i64>,
) -> Result<i64, String> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err("name is required".into());
    }
    let (variables_json, schema) = variables::store_schema(&form.variables)?;
    let now = Utc::now();

    let attached_vnode_id = optional_fk(&form.attached_vnode_id);
    if let Some(id) = id {
        let mut am: custom_source_doc_template::ActiveModel = TemplateEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("transaction template not found")?
            .into();
        am.name = Set(name.to_string());
        am.remarks = Set(optional_text(&form.remarks));
        am.variables = Set(variables_json);
        am.attached_vnode_id = Set(attached_vnode_id);
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        journal_template::save_template_lines(db, id, &form.journal_lines, &schema).await?;
        Ok(id)
    } else {
        let am = custom_source_doc_template::ActiveModel {
            name: Set(name.to_string()),
            remarks: Set(optional_text(&form.remarks)),
            variables: Set(variables_json),
            attached_vnode_id: Set(attached_vnode_id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let m = am.insert(db).await.map_err(|e| e.to_string())?;
        journal_template::save_template_lines(db, m.id, &form.journal_lines, &schema).await?;
        Ok(m.id)
    }
}

pub async fn list(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TemplateListQuery>,
) -> maud::Markup {
    let (templates, prepared) = load_template_rows(
        &state.db,
        uri.query().unwrap_or(""),
        q.sort.as_deref(),
        q.page.get(),
        q.page_size.get(),
    )
    .await;
    let page = CustomSourceDocTemplateListPage {
        templates,
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: can_mutate(),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets(CUSTOM_SOURCE_DOC_TEMPLATE_TABLE) {
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
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(t) = find_custom_source_doc_template_scoped(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocTemplateListRouteTag.url()).into_response();
    };
    let (attached_vnode_href, attached_vnode_label) =
        vnode_link(&state.db, t.attached_vnode_id).await;
    let page = CustomSourceDocTemplateDetailPage {
        id: t.id,
        name: t.name,
        remarks: t.remarks.unwrap_or_default(),
        attached_vnode_href,
        attached_vnode_label,
        variables: variables::schema_entries_from_stored(&t.variables).join("\n"),
        journal_lines: journal_template::lines_summary_display(&state.db, t.id).await,
        can_edit: can_mutate(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(_state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = CustomSourceDocTemplateCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        remarks: String::new(),
        attached_vnode_id: String::new(),
        attached_vnode_display: String::new(),
        variables: Vec::new(),
        journal_lines: journal_template::empty_lines_json(),
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
    HtmlFormBody(form): HtmlFormBody<CustomSourceDocTemplateForm>,
) -> Response {
    match save_from_form(&state.db, &form, None).await {
        Ok(id) => respond_create_modal_done(
            &htmx,
            CUSTOM_SOURCE_DOC_TEMPLATE_CREATE_MODAL,
            &q.refresh_table(),
            &CustomSourceDocTemplateDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let attached_vnode_display =
                vnode_display(&state.db, optional_fk(&form.attached_vnode_id)).await;
            let page = CustomSourceDocTemplateCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                name: form.name,
                remarks: form.remarks,
                attached_vnode_id: form.attached_vnode_id,
                attached_vnode_display,
                variables: form.variables,
                journal_lines: form.journal_lines,
                error: e,
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
    let Some(t) = find_custom_source_doc_template_scoped(&state.db, id).await else {
        return Redirect::to(&CustomSourceDocTemplateListRouteTag.url()).into_response();
    };
    let attached_vnode_display = vnode_display(&state.db, t.attached_vnode_id).await;
    let page = CustomSourceDocTemplateEditModalPage {
        id: t.id,
        form_name: q.form_name(),
        name: t.name,
        remarks: t.remarks.unwrap_or_default(),
        attached_vnode_id: optional_fk_form_value(t.attached_vnode_id),
        attached_vnode_display,
        variables: variables::schema_entries_from_stored(&t.variables),
        journal_lines: journal_template::lines_form_json(&state.db, t.id).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<CustomSourceDocTemplateForm>,
) -> Response {
    if find_custom_source_doc_template_scoped(&state.db, id)
        .await
        .is_none()
    {
        return Redirect::to(&CustomSourceDocTemplateListRouteTag.url()).into_response();
    }
    match save_from_form(&state.db, &form, Some(id)).await {
        Ok(_) => respond_edit_modal_done(
            &htmx,
            CUSTOM_SOURCE_DOC_TEMPLATE_EDIT_MODAL,
            &CustomSourceDocTemplateDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let attached_vnode_display =
                vnode_display(&state.db, optional_fk(&form.attached_vnode_id)).await;
            let page = CustomSourceDocTemplateEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                remarks: form.remarks,
                attached_vnode_id: form.attached_vnode_id,
                attached_vnode_display,
                variables: form.variables,
                journal_lines: form.journal_lines,
                error: e,
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
        modal_uid: CUSTOM_SOURCE_DOC_TEMPLATE_DELETE_MODAL.id().to_string(),
        message: "Are you sure you want to delete this transaction template?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_accounts.CustomSourceDocTemplateDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_custom_source_doc_template_scoped(&state.db, id)
        .await
        .is_none()
    {
        return Redirect::to(&CustomSourceDocTemplateListRouteTag.url()).into_response();
    }
    match TemplateEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&CustomSourceDocTemplateListRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete transaction template");
            let page = ConfirmDeletePage {
                modal_uid: CUSTOM_SOURCE_DOC_TEMPLATE_DELETE_MODAL.id().to_string(),
                message: "Are you sure you want to delete this transaction template?".into(),
                form_name: "p_finance_accounts.CustomSourceDocTemplateDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct TemplateSelectQuery {
    #[serde(flatten)]
    pub filter: TemplateListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

pub async fn select(
    Cap(state): Cap<AccountsState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TemplateSelectQuery>,
) -> maud::Markup {
    let (templates, prepared) = load_template_rows(
        &state.db,
        uri.query().unwrap_or(""),
        q.filter.sort.as_deref(),
        q.filter.page.get(),
        q.filter.page_size.get(),
    )
    .await;
    let page = CustomSourceDocTemplateSelectPage {
        templates,
        fields: prepared.fields,
        filters: prepared.filters,
        filter_errors: prepared.errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.unwrap_or_else(|| "TemplateID".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select(
        &htmx,
        CUSTOM_SOURCE_DOC_TEMPLATE_SELECT_TABLE,
        CUSTOM_SOURCE_DOC_TEMPLATE_SELECT_MODAL,
        &page,
    )
}
