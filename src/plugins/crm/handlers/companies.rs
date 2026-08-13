use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
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
    entities::company::{self, Entity as CompanyEntity},
    forms::CompanyForm,
    handlers::ModalNameQuery,
    keys::{
        CompanyCreateModalKey, CompanyEditModalKey, CompanySelectModalKey, CompanySelectTableKey,
        CompanyTableKey,
    },
    routes::CompanyDetailRouteTag,
    scope::{apply_company_filters, apply_company_sort, find_company_scoped, scope_superuser},
    state::CrmState,
    templates::{
        CompanyCreateModalPage, CompanyDetailPage, CompanyEditModalPage, CompanyListPage,
        CompanyRow, CompanySelectPage,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct CompanyListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct CompanySelectQuery {
    #[serde(flatten)]
    pub filter: CompanyListQuery,
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

async fn query_companies(
    db: &sea_orm::DatabaseConnection,
    q: &CompanyListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<company::Model>, u32, u64) {
    let mut query = CompanyEntity::find();
    query = apply_company_filters(query, q.name.as_deref());
    query = scope_superuser(query, auth);
    query = apply_company_sort(query, q.sort.as_deref());
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    (models, page, total)
}

fn model_to_row(c: company::Model) -> CompanyRow {
    CompanyRow {
        id: c.id,
        name: c.name,
        website: c.website.unwrap_or_default(),
    }
}

async fn load_company_rows(
    db: &sea_orm::DatabaseConnection,
    q: &CompanyListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> ObjectList<CompanyRow> {
    let (models, page, total) = query_companies(db, q, auth, page_size).await;
    let rows = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CompanyListQuery>,
) -> maud::Markup {
    let companies = load_company_rows(&state.db, &q, &ctx, PAGE_SIZE).await;
    let page = CompanyListPage {
        companies,
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: ctx.user.is_superuser,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<CompanyTableKey>() {
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
    let Some(company) = find_company_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/companies").into_response();
    };
    let page = CompanyDetailPage {
        id: company.id,
        name: company.name,
        address_line_1: company.address_line_1.unwrap_or_default(),
        address_line_2: company.address_line_2.unwrap_or_default(),
        city: company.city.unwrap_or_default(),
        pincode: company.pincode.unwrap_or_default(),
        state: company.state.unwrap_or_default(),
        website: company.website.unwrap_or_default(),
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
    let page = CompanyCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        address_line_1: String::new(),
        address_line_2: String::new(),
        city: String::new(),
        pincode: String::new(),
        state: String::new(),
        website: String::new(),
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
    Form(form): Form<CompanyForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/companies").into_response();
    }
    let now = Utc::now();
    let model = company::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(form.name.clone()),
        address_line_1: Set(opt_string(form.address_line_1.clone())),
        address_line_2: Set(opt_string(form.address_line_2.clone())),
        city: Set(opt_string(form.city.clone())),
        pincode: Set(opt_string(form.pincode.clone())),
        state: Set(opt_string(form.state.clone())),
        website: Set(opt_string(form.website.clone())),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk::<CompanyCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &CompanyDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = CompanyCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                target_input: q.target_input(),
                name: form.name,
                address_line_1: form.address_line_1,
                address_line_2: form.address_line_2,
                city: form.city,
                pincode: form.pincode,
                state: form.state,
                website: form.website,
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
        return Redirect::to("/crm/companies").into_response();
    }
    let Some(company) = find_company_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/companies").into_response();
    };
    let page = CompanyEditModalPage {
        id: company.id,
        form_name: q.form_name(),
        name: company.name,
        address_line_1: company.address_line_1.unwrap_or_default(),
        address_line_2: company.address_line_2.unwrap_or_default(),
        city: company.city.unwrap_or_default(),
        pincode: company.pincode.unwrap_or_default(),
        state: company.state.unwrap_or_default(),
        website: company.website.unwrap_or_default(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<CrmState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<CompanyForm>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/companies").into_response();
    }
    let Some(existing) = find_company_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/crm/companies").into_response();
    };
    let now = Utc::now();
    let mut am: company::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(form.name.clone());
    am.address_line_1 = Set(opt_string(form.address_line_1.clone()));
    am.address_line_2 = Set(opt_string(form.address_line_2.clone()));
    am.city = Set(opt_string(form.city.clone()));
    am.pincode = Set(opt_string(form.pincode.clone()));
    am.state = Set(opt_string(form.state.clone()));
    am.website = Set(opt_string(form.website.clone()));
    match am.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<CompanyEditModalKey>(
            &htmx,
            &CompanyDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = CompanyEditModalPage {
                id,
                form_name: q.form_name(),
                name: form.name,
                address_line_1: form.address_line_1,
                address_line_2: form.address_line_2,
                city: form.city,
                pincode: form.pincode,
                state: form.state,
                website: form.website,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !ctx.user.is_superuser {
        return Redirect::to("/crm/companies").into_response();
    }
    let _ = CompanyEntity::delete_by_id(id).exec(&state.db).await;
    Redirect::to("/crm/companies").into_response()
}

pub async fn select(
    Cap(state): Cap<CrmState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CompanySelectQuery>,
) -> maud::Markup {
    let companies = load_company_rows(&state.db, &q.filter, &ctx, PAGE_SIZE).await;
    let page = CompanySelectPage {
        companies,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "CompanyID".into()),
        can_edit: ctx.user.is_superuser,
    };
    respond_picker_select::<CompanySelectTableKey, CompanySelectModalKey, _>(&htmx, &page)
}
