use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use maud::html;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryOrder};

use crate::{
    components::{DEFAULT_PAGE_SIZE, ObjectList, SharedChromeFolder, SlotCtx},
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_fiscal_year::{entities::fiscal_year::{self, Entity as FiscalYearEntity}, forms::FiscalYearForm, handlers::ModalNameQuery, keys::{
        FiscalYearCreateModalKey, FiscalYearEditModalKey, FiscalYearSelectModalKey,
        FiscalYearSelectTableKey, FiscalYearTableKey,
    }, routes::FiscalYearDetailRouteTag, scope::{
        apply_fiscal_year_filters, find_fiscal_year_scoped, format_fiscal_date_input,
        model_to_row, parse_fiscal_date_end, parse_fiscal_date_start, scope_fiscal_years,
    }, state::FiscalYearState, templates::{
        FiscalYearCreateModalPage, FiscalYearDetailPage, FiscalYearEditModalPage, FiscalYearListPage,
        FiscalYearSelectPage,
    }};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct FiscalYearListQuery {
    #[serde(default, rename = "Code", alias = "code")]
    pub code: Option<String>,
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct FiscalYearSelectQuery {
    #[serde(flatten)]
    pub filter: FiscalYearListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

async fn query_fiscal_years(
    db: &sea_orm::DatabaseConnection,
    q: &FiscalYearListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> ObjectList<crate::plugins::finance_fiscal_year::templates::FiscalYearRow> {
    let mut query = FiscalYearEntity::find();
    query = apply_fiscal_year_filters(query, q.code.as_deref(), q.name.as_deref());
    query = scope_fiscal_years(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Code DESC") => query.order_by_desc(fiscal_year::Column::Code),
        s if s.eq_ignore_ascii_case("Code ASC") || s.eq_ignore_ascii_case("Code") => {
            query.order_by_asc(fiscal_year::Column::Code)
        }
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(fiscal_year::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(fiscal_year::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Start DESC") => {
            query.order_by_desc(fiscal_year::Column::StartsAt)
        }
        s if s.eq_ignore_ascii_case("Start ASC") || s.eq_ignore_ascii_case("Start") => {
            query.order_by_asc(fiscal_year::Column::StartsAt)
        }
        s if s.eq_ignore_ascii_case("End DESC") => query.order_by_desc(fiscal_year::Column::EndsAt),
        s if s.eq_ignore_ascii_case("End ASC") || s.eq_ignore_ascii_case("End") => {
            query.order_by_asc(fiscal_year::Column::EndsAt)
        }
        s if s.eq_ignore_ascii_case("Active DESC") => {
            query.order_by_desc(fiscal_year::Column::IsActive)
        }
        s if s.eq_ignore_ascii_case("Active ASC") || s.eq_ignore_ascii_case("Active") => {
            query.order_by_asc(fiscal_year::Column::IsActive)
        }
        _ => query
            .order_by_desc(fiscal_year::Column::StartsAt)
            .order_by_desc(fiscal_year::Column::Id),
    };

    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let rows: Vec<_> = models.into_iter().map(model_to_row).collect();
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<FiscalYearState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<FiscalYearListQuery>,
) -> maud::Markup {
    let fiscal_years = query_fiscal_years(&state.db, &q, &ctx, PAGE_SIZE).await;
    let page = FiscalYearListPage {
        fiscal_years,
        filter_code: q.code.clone().unwrap_or_default(),
        filter_name: q.name.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<FiscalYearTableKey>() {
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
    Cap(state): Cap<FiscalYearState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(fy) = find_fiscal_year_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-fiscal-years/").into_response();
    };
    let page = FiscalYearDetailPage {
        id: fy.id,
        code: fy.code,
        name: fy.name,
        start: format_fiscal_date_input(fy.starts_at),
        end: format_fiscal_date_input(fy.ends_at),
        is_active: fy.is_active,
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
        return html! { div class="alert alert-error" { "Forbidden" } };
    }
    let page = FiscalYearCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        code: String::new(),
        name: String::new(),
        start: String::new(),
        end: String::new(),
        is_active: true,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<FiscalYearState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<FiscalYearForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-fiscal-years/").into_response();
    }
    let now = Utc::now();
    let model = fiscal_year::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        code: Set(form.code.clone()),
        name: Set(form.name.clone()),
        starts_at: Set(parse_fiscal_date_start(&form.start)),
        ends_at: Set(parse_fiscal_date_end(&form.end)),
        is_active: Set(form.is_active),
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<FiscalYearCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &FiscalYearDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => {
            let page = FiscalYearCreateModalPage {
                form_name: q.form_name(),
                refresh_table: q.refresh_table(),
                code: form.code,
                name: form.name,
                start: form.start,
                end: form.end,
                is_active: form.is_active,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<FiscalYearState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-fiscal-years/").into_response();
    }
    let Some(fy) = find_fiscal_year_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-fiscal-years/").into_response();
    };
    let page = FiscalYearEditModalPage {
        id: fy.id,
        form_name: q.form_name(),
        code: fy.code,
        name: fy.name,
        start: format_fiscal_date_input(fy.starts_at),
        end: format_fiscal_date_input(fy.ends_at),
        is_active: fy.is_active,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<FiscalYearState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<FiscalYearForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-fiscal-years/").into_response();
    }
    let Some(existing) = find_fiscal_year_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-fiscal-years/").into_response();
    };
    let now = Utc::now();
    let model = fiscal_year::ActiveModel {
        id: Set(existing.id),
        updated_at: Set(Some(now)),
        code: Set(form.code.clone()),
        name: Set(form.name.clone()),
        starts_at: Set(parse_fiscal_date_start(&form.start)),
        ends_at: Set(parse_fiscal_date_end(&form.end)),
        is_active: Set(form.is_active),
        ..Default::default()
    };
    match model.update(&state.db).await {
        Ok(_) => respond_edit_modal_done::<FiscalYearEditModalKey>(
            &htmx,
            &FiscalYearDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = FiscalYearEditModalPage {
                id,
                form_name: q.form_name(),
                code: form.code,
                name: form.name,
                start: form.start,
                end: form.end,
                is_active: form.is_active,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<FiscalYearState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-fiscal-years/").into_response();
    }
    if find_fiscal_year_scoped(&state.db, id, &ctx).await.is_some() {
        let _ = fiscal_year::Entity::delete_by_id(id).exec(&state.db).await;
    }
    Redirect::to("/finance-fiscal-years/").into_response()
}

pub async fn select(
    Cap(state): Cap<FiscalYearState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<FiscalYearSelectQuery>,
) -> maud::Markup {
    let fiscal_years = query_fiscal_years(&state.db, &q.filter, &ctx, PAGE_SIZE).await;
    let page = FiscalYearSelectPage {
        fiscal_years,
        filter_code: q.filter.code.clone().unwrap_or_default(),
        filter_name: q.filter.name.clone().unwrap_or_default(),
        target_input: q
            .target_input
            .clone()
            .unwrap_or_else(|| "FiscalYearID".into()),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
    };
    respond_picker_select::<FiscalYearSelectTableKey, FiscalYearSelectModalKey, _>(&htmx, &page)
}
