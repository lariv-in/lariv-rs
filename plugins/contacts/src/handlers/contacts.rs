use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};

use lariv_core::components::{FilterClauseView, ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::{HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::list_filters::{
    CmpOp, FilterClause, FilterField, JoinOp, apply_filters, foreign_key_column,
    parse_list_filters, text_column, with_search_key,
};
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done_fk_extra, respond_edit_modal_done,
};
use lariv_plugin_users::{middleware::RequireAuth, state::AuthContext};

use crate::{
    entities::{
        company::{self, Entity as CompanyEntity},
        contact::{self, Entity as ContactEntity},
    },
    forms::ContactForm,
    handlers::ModalNameQuery,
    keys::{
        ContactCreateModalKey, ContactDeleteModalKey, ContactEditModalKey, ContactSelectModalKey,
        ContactSelectTableKey, ContactTableKey,
    },
    routes::{CompanyFkSelectRouteTag, ContactDefaultRouteTag, ContactDetailRouteTag},
    scope::{apply_contact_sort, company_display_label, find_company_scoped, find_contact_scoped},
    state::ContactsState,
    templates::{
        ConfirmDeletePage, ContactCreateModalPage, ContactDetailPage, ContactEditModalPage,
        ContactListPage, ContactRow, ContactSelectPage,
    },
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct ContactListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct ContactSelectQuery {
    #[serde(flatten)]
    pub filter: ContactListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn opt_string(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

fn checkbox_on(raw: &str) -> bool {
    raw == "on" || raw == "true" || raw == "1"
}

fn contact_fields() -> Vec<FilterField> {
    vec![
        with_search_key(
            foreign_key_column(
                "company_id",
                "Company",
                contact::Column::CompanyId,
                format!("{}?filter.0.field=name", CompanyFkSelectRouteTag.url()),
                "Any company…",
            ),
            "filter.0.value",
        ),
        text_column("name", "Name", contact::Column::Name),
    ]
}

/// Picker typeahead still submits `Name`. Indexed `filter.*` clauses win when present.
fn clauses_from_query(query: &str, typeahead: bool) -> Vec<FilterClause> {
    let mut clauses = parse_list_filters(query);
    if !clauses.is_empty() || !typeahead {
        return clauses;
    }
    let Some(name) = UrlencodedFields::parse(query.as_bytes())
        .ok()
        .and_then(|fields| fields.get_first("Name").map(str::to_string))
        .filter(|name| !name.trim().is_empty())
    else {
        return clauses;
    };
    clauses.push(FilterClause {
        index: 0,
        join: JoinOp::And,
        field: "name".into(),
        op: CmpOp::Eq,
        values: vec![name.trim().to_string()],
    });
    clauses
}

async fn fill_company_display(db: &sea_orm::DatabaseConnection, rows: &mut [FilterClauseView]) {
    for row in rows {
        if row.clause.field != "company_id" {
            continue;
        }
        let Ok(id) = row.clause.value().parse::<i64>() else {
            continue;
        };
        if id > 0 {
            row.fk_display = company_display_label(db, id).await;
        }
    }
}

fn optional_fk(id: i64) -> Option<i64> {
    if id > 0 { Some(id) } else { None }
}

fn contacts_list_url() -> String {
    ContactDefaultRouteTag.url()
}

async fn query_contacts(
    db: &sea_orm::DatabaseConnection,
    q: &ContactListQuery,
    _auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> (Vec<contact::Model>, u32, u64) {
    let mut query = scope_allowed::<super::super::routes::ContactsView, _>(ContactEntity::find());
    query = query.filter(condition);
    query = apply_contact_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

async fn company_names(
    db: &sea_orm::DatabaseConnection,
    ids: &[i64],
) -> std::collections::HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return std::collections::HashMap::new();
    }
    CompanyEntity::find()
        .filter(company::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect()
}

fn model_to_row(c: contact::Model, company: String) -> ContactRow {
    ContactRow {
        id: c.id,
        company_id: c.company_id.unwrap_or(0),
        company,
        name: c.display_name(),
        email: c.email.unwrap_or_default(),
        phone: c.phone.unwrap_or_default(),
        is_primary: c.is_primary,
    }
}

async fn load_contact_rows(
    db: &sea_orm::DatabaseConnection,
    q: &ContactListQuery,
    auth: &AuthContext,
    page_size: u32,
    condition: Condition,
) -> ObjectList<ContactRow> {
    let (models, page, total) = query_contacts(db, q, auth, page_size, condition).await;
    let names = company_names(
        db,
        &models
            .iter()
            .filter_map(|c| c.company_id)
            .collect::<Vec<_>>(),
    )
    .await;
    let rows = models
        .into_iter()
        .map(|c| {
            let company = c
                .company_id
                .and_then(|id| names.get(&id).cloned())
                .unwrap_or_default();
            model_to_row(c, company)
        })
        .collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<ContactsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ContactListQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), false);
    let fields = contact_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let contacts =
        load_contact_rows(&state.db, &q, &ctx, q.page_size.get(), applied.condition).await;
    let mut filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    fill_company_display(&state.db, &mut filters).await;
    let page = ContactListPage {
        contacts,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<ContactTableKey>() {
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
    Cap(state): Cap<ContactsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(contact) = find_contact_scoped(&state.db, id).await else {
        return Redirect::to(&contacts_list_url()).into_response();
    };
    let company_id = contact.company_id.unwrap_or(0);
    let page = ContactDetailPage {
        id: contact.id,
        company_id,
        company: company_display_label(&state.db, company_id).await,
        display_name: contact.display_name(),
        email: contact.email.unwrap_or_default(),
        phone: contact.phone.unwrap_or_default(),
        is_primary: contact.is_primary,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let page = ContactCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        company_id: 0,
        company_display: String::new(),
        name: String::new(),
        email: String::new(),
        phone: String::new(),
        is_primary: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

async fn resolve_optional_company(
    db: &sea_orm::DatabaseConnection,
    _auth: &AuthContext,
    company_id: i64,
) -> Result<Option<i64>, String> {
    let Some(id) = optional_fk(company_id) else {
        return Ok(None);
    };
    if find_company_scoped(db, id).await.is_none() {
        return Err("company not found".to_string());
    }
    Ok(Some(id))
}

pub async fn create_post(
    Cap(state): Cap<ContactsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ContactForm>,
) -> Response {
    let company_id = form.company_id;
    let resolved = match resolve_optional_company(&state.db, &ctx, company_id).await {
        Ok(id) => id,
        Err(error) => {
            let page = ContactCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                company_id,
                company_display: company_display_label(&state.db, company_id).await,
                name: form.name,
                email: form.email,
                phone: form.phone,
                is_primary: form.is_primary,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let now = Utc::now();
    let model = contact::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_id: Set(resolved),
        name: Set(form.name.clone()),
        email: Set(opt_string(form.email.clone())),
        phone: Set(opt_string(form.phone.clone())),
        is_primary: Set(checkbox_on(&form.is_primary)),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            let display = saved.display_name();
            let company_id = saved.company_id.unwrap_or(0);
            let company_name = company_display_label(&state.db, company_id).await;
            let company_id_s = company_id.to_string();
            respond_create_modal_done_fk_extra::<ContactCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &ContactDetailRouteTag::new(saved.id).url(),
                saved.id,
                &display,
                &q.target_input(),
                &[
                    ("company_id", company_id_s.as_str()),
                    ("company_name", company_name.as_str()),
                ],
            )
        }
        Err(e) => {
            let page = ContactCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                company_id,
                company_display: company_display_label(&state.db, company_id).await,
                name: form.name,
                email: form.email,
                phone: form.phone,
                is_primary: form.is_primary,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<ContactsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(contact) = find_contact_scoped(&state.db, id).await else {
        return Redirect::to(&contacts_list_url()).into_response();
    };
    let company_id = contact.company_id.unwrap_or(0);
    let page = ContactEditModalPage {
        id: contact.id,
        form_name: q.form_name(),
        company_id,
        company_display: company_display_label(&state.db, company_id).await,
        name: contact.name,
        email: contact.email.unwrap_or_default(),
        phone: contact.phone.unwrap_or_default(),
        is_primary: if contact.is_primary {
            "on".into()
        } else {
            String::new()
        },
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn contact_edit_modal_error(
    db: &sea_orm::DatabaseConnection,
    chrome: &SharedChromeFolder,
    ctx: &AuthContext,
    id: i64,
    q: &ModalNameQuery,
    form: &ContactForm,
    error: &str,
) -> Response {
    let page = ContactEditModalPage {
        id,
        form_name: q.form_name(),
        company_id: form.company_id,
        company_display: company_display_label(db, form.company_id).await,
        name: form.name.clone(),
        email: form.email.clone(),
        phone: form.phone.clone(),
        is_primary: form.is_primary.clone(),
        error: error.to_string(),
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<ContactsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ContactForm>,
) -> Response {
    let Some(existing) = find_contact_scoped(&state.db, id).await else {
        return Redirect::to(&contacts_list_url()).into_response();
    };
    let company_id = form.company_id;
    let resolved = match resolve_optional_company(&state.db, &ctx, company_id).await {
        Ok(id) => id,
        Err(error) => {
            return contact_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &error).await;
        }
    };
    let now = Utc::now();
    let mut am: contact::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.company_id = Set(resolved);
    am.name = Set(form.name.clone());
    am.email = Set(opt_string(form.email.clone()));
    am.phone = Set(opt_string(form.phone.clone()));
    am.is_primary = Set(checkbox_on(&form.is_primary));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<ContactEditModalKey>(
            &htmx,
            &ContactDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            contact_edit_modal_error(&state.db, &chrome, &ctx, id, &q, &form, &e.to_string()).await
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
        modal_uid: ContactDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this contact?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_contacts.ContactDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<ContactsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_contact_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&contacts_list_url()).into_response();
    }
    match ContactEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&contacts_list_url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete contact");
            let page = ConfirmDeletePage {
                modal_uid: ContactDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this contact?".into(),
                form_name: "p_contacts.ContactDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<ContactsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ContactSelectQuery>,
) -> maud::Markup {
    let clauses = clauses_from_query(uri.query().unwrap_or(""), true);
    let fields = contact_fields();
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let contacts = load_contact_rows(
        &state.db,
        &q.filter,
        &ctx,
        q.filter.page_size.get(),
        applied.condition,
    )
    .await;
    let mut filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    fill_company_display(&state.db, &mut filters).await;
    let page = ContactSelectPage {
        contacts,
        fields,
        filters,
        filter_errors: applied.errors,
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "ContactID".into()),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select::<ContactSelectTableKey, ContactSelectModalKey, _>(&htmx, &page)
}

#[cfg(test)]
mod tests {
    use super::clauses_from_query;

    #[test]
    fn filter_query_accepts_the_indexed_field_name() {
        let clauses = clauses_from_query("filter.0.field=company_id&filter.0.value=8", false);
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].field, "company_id");
        assert_eq!(clauses[0].value(), "8");
    }
}
