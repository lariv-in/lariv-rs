use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait};

use crate::{
    html_form::HtmlFormBody,
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx, SwapKey},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};

use crate::plugins::crm::{
    entities::contact::{self, Entity as ContactEntity},
    forms::ContactForm,
    handlers::ModalNameQuery,
    keys::{
        ContactCreateModalKey, ContactDeleteModalKey, ContactEditModalKey, ContactSelectModalKey,
        ContactSelectTableKey, ContactTableKey,
    },
    routes::ContactDetailRouteTag,
    scope::{
        apply_contact_filters, apply_contact_sort, company_display_label, find_company_scoped,
        find_contact_scoped, scope_superuser,
    },
    state::CrmState,
    templates::{
        ConfirmDeletePage, ContactCreateModalPage, ContactDetailPage, ContactEditModalPage,
        ContactListPage, ContactRow, ContactSelectPage,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct ContactListQuery {
    #[serde(default, rename = "CompanyId", alias = "company_id")]
    pub company_id: Option<String>,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
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

fn parse_company_id(raw: &str) -> Option<i64> {
    raw.trim().parse().ok().filter(|id| *id > 0)
}

async fn filter_company_display(
    db: &sea_orm::DatabaseConnection,
    company_id: Option<&str>,
) -> String {
    let Some(id) = company_id.and_then(parse_company_id) else {
        return String::new();
    };
    company_display_label(db, id).await
}

async fn query_contacts(
    db: &sea_orm::DatabaseConnection,
    q: &ContactListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<contact::Model>, u32, u64) {
    let company_id = q.company_id.as_deref().and_then(parse_company_id);
    let mut query = ContactEntity::find();
    query = apply_contact_filters(query, company_id, q.name.as_deref());
    query = scope_superuser(query, auth);
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

fn model_to_row(c: contact::Model) -> ContactRow {
    ContactRow {
        id: c.id,
        company_id: c.company_id,
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
) -> ObjectList<ContactRow> {
    let (models, page, total) = query_contacts(db, q, auth, page_size).await;
    let rows = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ContactListQuery>,
) -> maud::Markup {
    let contacts = load_contact_rows(&state.db, &q, &ctx, PAGE_SIZE).await;
    let page = ContactListPage {
        contacts,
        filter_company_id: q.company_id.clone().unwrap_or_default(),
        filter_company_display: filter_company_display(&state.db, q.company_id.as_deref()).await,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
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
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(contact) = find_contact_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/contacts").into_response();
    };
    let page = ContactDetailPage {
        id: contact.id,
        company_id: contact.company_id,
        display_name: contact.display_name(),
        email: contact.email.unwrap_or_default(),
        phone: contact.phone.unwrap_or_default(),
        is_primary: contact.is_primary,
        can_edit: ctx.user.is_superuser,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !ctx.user.is_superuser {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
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

pub async fn create_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ContactForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/contacts").into_response();
    }
    let company_id = form.company_id;
    if company_id <= 0 {
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
            error: "company is required".to_string(),
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    if find_company_scoped(&state.db, company_id, &ctx)
        .await
        .is_none()
    {
        return Redirect::to("/crm/contacts").into_response();
    }
    let now = Utc::now();
    let model = contact::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        company_id: Set(company_id),
        name: Set(form.name.clone()),
        email: Set(opt_string(form.email.clone())),
        phone: Set(opt_string(form.phone.clone())),
        is_primary: Set(checkbox_on(&form.is_primary)),
    };
    match model.insert(&state.db).await {
        Ok(saved) => {
            let display = saved.display_name();
            respond_create_modal_done_fk::<ContactCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &ContactDetailRouteTag::new(saved.id).url(),
                saved.id,
                &display,
                &q.target_input(),
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
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/contacts").into_response();
    }
    let Some(contact) = find_contact_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/contacts").into_response();
    };
    let page = ContactEditModalPage {
        id: contact.id,
        form_name: q.form_name(),
        company_id: contact.company_id,
        company_display: company_display_label(&state.db, contact.company_id).await,
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
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ContactForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/contacts").into_response();
    }
    let Some(existing) = find_contact_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/contacts").into_response();
    };
    let company_id = form.company_id;
    if company_id <= 0 {
        return contact_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "company is required",
        )
        .await;
    }
    if find_company_scoped(&state.db, company_id, &ctx)
        .await
        .is_none()
    {
        return contact_edit_modal_error(
            &state.db,
            &chrome,
            &ctx,
            id,
            &q,
            &form,
            "company is required",
        )
        .await;
    }
    let now = Utc::now();
    let mut am: contact::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.company_id = Set(company_id);
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
            .unwrap_or_else(|| "p_crm.ContactDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/contacts").into_response();
    }
    match ContactEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect("/crm/contacts"),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete contact");
            let page = ConfirmDeletePage {
                modal_uid: ContactDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this contact?".into(),
                form_name: "p_crm.ContactDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ContactSelectQuery>,
) -> maud::Markup {
    let contacts = load_contact_rows(&state.db, &q.filter, &ctx, PAGE_SIZE).await;
    let page = ContactSelectPage {
        contacts,
        filter_company_id: q.filter.company_id.clone().unwrap_or_default(),
        filter_company_display: filter_company_display(&state.db, q.filter.company_id.as_deref())
            .await,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "ContactID".into()),
        can_edit: ctx.user.is_superuser,
    };
    respond_picker_select::<ContactSelectTableKey, ContactSelectModalKey, _>(&htmx, &page)
}
