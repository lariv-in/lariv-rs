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
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done_fk, respond_edit_modal_done,
    },
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::{
    entities::currency::{self, Entity as CurrencyEntity},
    forms::CurrencyForm,
    handlers::ModalNameQuery,
    keys::{
        CurrencyCreateModalKey, CurrencyEditModalKey, CurrencySelectModalKey,
        CurrencySelectTableKey, CurrencyTableKey,
    },
    routes::{CurrencyDetailRouteTag, CurrencyListRouteTag},
    scope::{apply_currency_filters, find_currency_scoped, scope_superuser},
    state::AccountsState,
    templates::{
        CurrencyCreateModalPage, CurrencyDetailPage, CurrencyEditModalPage, CurrencyListPage,
        CurrencyRow, CurrencySelectPage,
    },
};

use super::util::{parse_i32, path_and_query};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct CurrencyListQuery {
    #[serde(default, rename = "Code", alias = "code")]
    pub code: Option<String>,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "Symbol", alias = "symbol")]
    pub symbol: Option<String>,
    #[serde(default, rename = "MinorUnit", alias = "minor_unit")]
    pub minor_unit: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct CurrencySelectQuery {
    #[serde(flatten)]
    pub filter: CurrencyListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

async fn load_currency_rows(
    db: &sea_orm::DatabaseConnection,
    q: &CurrencyListQuery,
    auth: &AuthContext,
) -> ObjectList<CurrencyRow> {
    let mut query = CurrencyEntity::find();
    query = apply_currency_filters(
        query,
        q.code.as_deref(),
        q.name.as_deref(),
        q.symbol.as_deref(),
        q.minor_unit.as_deref(),
    );
    query = scope_superuser(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Code DESC") => query.order_by_desc(currency::Column::Code),
        s if s.eq_ignore_ascii_case("Code ASC") || s.eq_ignore_ascii_case("Code") => {
            query.order_by_asc(currency::Column::Code)
        }
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(currency::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(currency::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Symbol DESC") => query.order_by_desc(currency::Column::Symbol),
        s if s.eq_ignore_ascii_case("Symbol ASC") || s.eq_ignore_ascii_case("Symbol") => {
            query.order_by_asc(currency::Column::Symbol)
        }
        s if s.eq_ignore_ascii_case("MinorUnit DESC") => {
            query.order_by_desc(currency::Column::MinorUnit)
        }
        s if s.eq_ignore_ascii_case("MinorUnit ASC") || s.eq_ignore_ascii_case("MinorUnit") => {
            query.order_by_asc(currency::Column::MinorUnit)
        }
        _ => query.order_by_asc(currency::Column::Code),
    };
    let page = q.page.get();
    let paginator = query.paginate(db, PAGE_SIZE as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows: Vec<CurrencyRow> = models
        .into_iter()
        .map(|c| CurrencyRow {
            id: c.id,
            code: c.code,
            name: c.name,
            symbol: c.symbol,
            minor_unit: c.minor_unit,
        })
        .collect();
    ObjectList::from_page(rows, page, PAGE_SIZE, total)
}

pub async fn list(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CurrencyListQuery>,
) -> maud::Markup {
    let currencies = load_currency_rows(&state.db, &q, &ctx).await;
    let page = CurrencyListPage {
        currencies,
        filter_code: q.code.clone().unwrap_or_default(),
        filter_name: q.name.clone().unwrap_or_default(),
        filter_symbol: q.symbol.clone().unwrap_or_default(),
        filter_minor_unit: q.minor_unit.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<CurrencyTableKey>() {
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
    let Some(c) = find_currency_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    };
    let page = CurrencyDetailPage {
        id: c.id,
        code: c.code,
        name: c.name,
        symbol: c.symbol,
        minor_unit: c.minor_unit,
        can_edit: require_superuser(&ctx),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !require_superuser(&ctx) {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = CurrencyCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        code: String::new(),
        name: String::new(),
        symbol: String::new(),
        minor_unit: String::new(),
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
    Form(form): Form<CurrencyForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    }
    let now = Utc::now();
    let model = currency::ActiveModel {
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        code: Set(parse_i32(&form.code).unwrap_or(0)),
        name: Set(form.name.clone()),
        symbol: Set(form.symbol.clone()),
        minor_unit: Set(parse_i32(&form.minor_unit).unwrap_or(0)),
        ..Default::default()
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done_fk::<CurrencyCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &CurrencyDetailRouteTag::new(saved.id).url(),
            saved.id,
            &saved.name,
            &q.target_input(),
        ),
        Err(e) => {
            let page = CurrencyCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                code: form.code,
                name: form.name,
                symbol: form.symbol,
                minor_unit: form.minor_unit,
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
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    }
    let Some(c) = find_currency_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    };
    let page = CurrencyEditModalPage::from_model(&c, q.form_name());
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<AccountsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<CurrencyForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    }
    let Some(existing) = find_currency_scoped(&state.db, id, &ctx).await else {
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    };
    let now = Utc::now();
    let model = currency::ActiveModel {
        id: Set(existing.id),
        updated_at: Set(Some(now)),
        code: Set(parse_i32(&form.code).unwrap_or(existing.code)),
        name: Set(form.name.clone()),
        symbol: Set(form.symbol.clone()),
        minor_unit: Set(parse_i32(&form.minor_unit).unwrap_or(existing.minor_unit)),
        ..Default::default()
    };
    match model.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<CurrencyEditModalKey>(
            &htmx,
            &CurrencyDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = CurrencyEditModalPage {
                id,
                form_name: q.form_name(),
                code: form.code,
                name: form.name,
                symbol: form.symbol,
                minor_unit: form.minor_unit,
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
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    }
    if find_currency_scoped(&state.db, id, &ctx).await.is_none() {
        return Redirect::to(&CurrencyListRouteTag.url()).into_response();
    }
    let _ = currency::Entity::delete_by_id(id).exec(&state.db).await;
    Redirect::to(&CurrencyListRouteTag.url()).into_response()
}

pub async fn select(
    Cap(state): Cap<AccountsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<CurrencySelectQuery>,
) -> maud::Markup {
    let currencies = load_currency_rows(&state.db, &q.filter, &ctx).await;
    let page = CurrencySelectPage {
        currencies,
        filter_code: q.filter.code.clone().unwrap_or_default(),
        filter_name: q.filter.name.clone().unwrap_or_default(),
        filter_symbol: q.filter.symbol.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.unwrap_or_else(|| "CurrencyId".into()),
    };
    respond_picker_select::<CurrencySelectTableKey, CurrencySelectModalKey, _>(&htmx, &page)
}
