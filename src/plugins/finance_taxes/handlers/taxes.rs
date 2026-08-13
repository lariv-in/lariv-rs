use axum::{
    Form,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use rust_decimal::Decimal;
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

use crate::plugins::finance_taxes::{
    entities::tax::{self, Entity as TaxEntity, TaxKind},
    forms::{TaxForm, tax_type_label},
    handlers::ModalNameQuery,
    keys::{
        TaxCreateModalKey, TaxEditModalKey, TaxMultiSelectModalKey, TaxMultiSelectTableKey,
        TaxTableKey,
    },
    routes::TaxDetailRouteTag,
    scope::{account_label, apply_tax_filters, find_tax_scoped, model_to_row, scope_taxes},
    state::TaxesState,
    templates::{
        TaxCreateModalPage, TaxDetailPage, TaxEditModalPage, TaxListPage, TaxMultiSelectPage,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaxListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "TaxType", alias = "tax_type")]
    pub tax_type: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct TaxSelectQuery {
    #[serde(flatten)]
    pub filter: TaxListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn parse_account_id(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        s.parse().ok().filter(|&id| id > 0)
    }
}

fn parse_percentage(s: &str) -> Option<Decimal> {
    crate::plugins::finance_common::decimal::parse_decimal(s)
}

fn validate_tax(tax_type: TaxKind, account_id: Option<i64>) -> bool {
    if tax_type == TaxKind::Withholding {
        account_id.filter(|&id| id > 0).is_some()
    } else {
        true
    }
}

async fn query_taxes(
    db: &sea_orm::DatabaseConnection,
    q: &TaxListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> ObjectList<crate::plugins::finance_taxes::templates::TaxRow> {
    let mut query = TaxEntity::find();
    query = apply_tax_filters(query, q.name.as_deref(), q.tax_type.as_deref());
    query = scope_taxes(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(tax::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(tax::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Type DESC") => query.order_by_desc(tax::Column::TaxType),
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(tax::Column::TaxType)
        }
        s if s.eq_ignore_ascii_case("Percentage DESC") => {
            query.order_by_desc(tax::Column::Percentage)
        }
        s if s.eq_ignore_ascii_case("Percentage ASC") || s.eq_ignore_ascii_case("Percentage") => {
            query.order_by_asc(tax::Column::Percentage)
        }
        _ => query
            .order_by_desc(tax::Column::CreatedAt)
            .order_by_desc(tax::Column::Id),
    };
    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(models.len());
    for model in models {
        rows.push(model_to_row(db, model).await);
    }
    ObjectList::from_page(rows, page, page_size, total)
}

pub async fn list(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaxListQuery>,
) -> maud::Markup {
    let taxes = query_taxes(&state.db, &q, &ctx, PAGE_SIZE).await;
    let page = TaxListPage {
        taxes,
        filter_name: q.name.clone().unwrap_or_default(),
        filter_tax_type: q.tax_type.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<TaxTableKey>() {
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
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(t) = find_tax_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-taxes/").into_response();
    };
    let page = TaxDetailPage {
        id: t.id,
        name: t.name,
        tax_type: tax_type_label(&t.tax_type),
        percentage: t.percentage.normalize().to_string(),
        account_label: account_label(&state.db, t.account_id).await,
        can_edit: require_superuser(&ctx),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-taxes/").into_response();
    }
    let page = TaxCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        tax_type: TaxKind::Levied.as_str().to_string(),
        percentage: String::new(),
        account_id: String::new(),
        account_display: String::new(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<TaxForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-taxes/").into_response();
    }
    let account_display = account_label(&state.db, parse_account_id(&form.account_id)).await;
    let render_error = |error: String| {
        let page = TaxCreateModalPage {
            form_name: q.form_name(),
            refresh_table: q.refresh_table(),
            name: form.name.clone(),
            tax_type: form.tax_type.clone(),
            percentage: form.percentage.clone(),
            account_id: form.account_id.clone(),
            account_display: account_display.clone(),
            error,
        };
        html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
    };
    let Some(tax_type) = TaxKind::parse(&form.tax_type) else {
        return render_error("Invalid tax type".into());
    };
    let Some(percentage) = parse_percentage(&form.percentage) else {
        return render_error("Invalid percentage".into());
    };
    let account_id = parse_account_id(&form.account_id);
    if !validate_tax(tax_type, account_id) {
        return render_error("Withholding taxes require an account".into());
    }
    let now = Utc::now();
    let model = tax::ActiveModel {
        name: Set(form.name.clone()),
        tax_type: Set(tax_type),
        percentage: Set(percentage),
        account_id: Set(account_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    match model.insert(&state.db).await {
        Ok(saved) => respond_create_modal_done::<TaxCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &TaxDetailRouteTag::new(saved.id).url(),
        ),
        Err(e) => render_error(e.to_string()),
    }
}

pub async fn edit_get(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-taxes/").into_response();
    }
    let Some(t) = find_tax_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-taxes/").into_response();
    };
    let page = TaxEditModalPage {
        id: t.id,
        form_name: q.form_name(),
        name: t.name,
        tax_type: t.tax_type.as_str().to_string(),
        percentage: t.percentage.normalize().to_string(),
        account_id: t.account_id.map(|id| id.to_string()).unwrap_or_default(),
        account_display: account_label(&state.db, t.account_id).await,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

fn tax_edit_modal_page_from_form(
    id: i64,
    form: &TaxForm,
    form_name: String,
    account_display: String,
    error: String,
) -> TaxEditModalPage {
    TaxEditModalPage {
        id,
        form_name,
        name: form.name.clone(),
        tax_type: form.tax_type.clone(),
        percentage: form.percentage.clone(),
        account_id: form.account_id.clone(),
        account_display,
        error,
    }
}

pub async fn edit_post(
    Cap(state): Cap<TaxesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    Form(form): Form<TaxForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-taxes/").into_response();
    }
    let Some(existing) = find_tax_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-taxes/").into_response();
    };
    let account_display = account_label(&state.db, parse_account_id(&form.account_id)).await;
    let Some(tax_type) = TaxKind::parse(&form.tax_type) else {
        let page = tax_edit_modal_page_from_form(
            id,
            &form,
            q.form_name(),
            account_display,
            "invalid tax type".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let Some(percentage) = parse_percentage(&form.percentage) else {
        let page = tax_edit_modal_page_from_form(
            id,
            &form,
            q.form_name(),
            account_display,
            "invalid percentage".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    };
    let account_id = parse_account_id(&form.account_id);
    if !validate_tax(tax_type, account_id) {
        let page = tax_edit_modal_page_from_form(
            id,
            &form,
            q.form_name(),
            account_display,
            "invalid tax configuration".into(),
        );
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    let model = tax::ActiveModel {
        id: Set(existing.id),
        name: Set(form.name.clone()),
        tax_type: Set(tax_type),
        percentage: Set(percentage),
        account_id: Set(account_id),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    match model.update(&state.db).await {
        Ok(_) => {
            respond_edit_modal_done::<TaxEditModalKey>(&htmx, &TaxDetailRouteTag::new(id).url())
        }
        Err(e) => {
            let page = tax_edit_modal_page_from_form(
                id,
                &form,
                q.form_name(),
                account_display,
                e.to_string(),
            );
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<TaxesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-taxes/").into_response();
    }
    if find_tax_scoped(&state.db, id, &ctx).await.is_some() {
        let _ = tax::Entity::delete_by_id(id).exec(&state.db).await;
    }
    Redirect::to("/finance-taxes/").into_response()
}

pub async fn multi_select(
    Cap(state): Cap<TaxesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<TaxSelectQuery>,
) -> maud::Markup {
    let taxes = query_taxes(&state.db, &q.filter, &ctx, PAGE_SIZE).await;
    let page = TaxMultiSelectPage {
        taxes,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        filter_tax_type: q.filter.tax_type.clone().unwrap_or_default(),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        target_input: q.target_input.clone().unwrap_or_else(|| "TaxIds".into()),
        can_edit: require_superuser(&ctx),
    };
    respond_picker_select::<TaxMultiSelectTableKey, TaxMultiSelectModalKey, _>(&htmx, &page)
}
