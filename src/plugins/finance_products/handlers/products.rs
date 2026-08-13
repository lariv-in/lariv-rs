use crate::plugins::finance_common::decimal::{self, parse_decimal};
use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, QueryOrder};
use serde::Deserialize;

use crate::{
    components::{DEFAULT_PAGE_SIZE, ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    picker::respond_picker_select,
    plugins::users::{middleware::RequireAuth, state::AuthContext},
    template::RenderAppPane,
    web::{
        Htmx, QueryPage, html_built_page_or_app_layout, html_built_page_with_slots,
        respond_create_modal_done, respond_edit_modal_done,
    },
};

use crate::plugins::finance_accounts::scope::load_default_currency_format;
use crate::plugins::finance_common::require_superuser;
use crate::plugins::finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::plugins::finance_products::{
    entities::product::{self, Entity as ProductEntity, ProductType},
    forms::ProductForm,
    handlers::ModalNameQuery,
    keys::{
        ProductCreateModalKey, ProductEditModalKey, ProductSelectModalKey, ProductSelectTableKey,
        ProductTableKey,
    },
    preferences::{load_default_product_tax_ids, load_product_tax_ids, set_product_tax_ids},
    routes::ProductDetailRouteTag,
    scope::{apply_product_filters, find_product_scoped, scope_products},
    state::ProductsState,
    templates::{
        ProductCreateModalPage, ProductDetailPage, ProductEditModalPage, ProductListPage,
        ProductRow, ProductSelectPage,
    },
};

const PAGE_SIZE: u32 = DEFAULT_PAGE_SIZE;

#[derive(Debug, Deserialize, Default)]
pub struct ProductListQuery {
    #[serde(default, rename = "Name", alias = "name")]
    pub name: Option<String>,
    #[serde(default, rename = "Reference", alias = "reference")]
    pub reference: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct ProductSelectQuery {
    #[serde(flatten)]
    pub filter: ProductListQuery,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

async fn tax_items_from_ids(
    db: &sea_orm::DatabaseConnection,
    tax_ids: &[i64],
) -> Vec<ManyToManyItem> {
    load_taxes_by_ids(db, tax_ids)
        .await
        .unwrap_or_default()
        .iter()
        .map(|t| ManyToManyItem {
            key: t.id.to_string(),
            value: tax_label(t),
        })
        .collect()
}

async fn query_products(
    db: &sea_orm::DatabaseConnection,
    q: &ProductListQuery,
    auth: &AuthContext,
    page_size: u32,
) -> (Vec<ProductRow>, u32, u64) {
    let mut query = ProductEntity::find();
    query = apply_product_filters(query, q.name.as_deref(), q.reference.as_deref());
    query = scope_products(query, auth);
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(product::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(product::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Type DESC") => {
            query.order_by_desc(product::Column::ProductType)
        }
        s if s.eq_ignore_ascii_case("Type ASC") || s.eq_ignore_ascii_case("Type") => {
            query.order_by_asc(product::Column::ProductType)
        }
        s if s.eq_ignore_ascii_case("Reference DESC") => {
            query.order_by_desc(product::Column::Reference)
        }
        s if s.eq_ignore_ascii_case("Reference ASC") || s.eq_ignore_ascii_case("Reference") => {
            query.order_by_asc(product::Column::Reference)
        }
        s if s.eq_ignore_ascii_case("BaseCost DESC") => {
            query.order_by_desc(product::Column::BaseCost)
        }
        s if s.eq_ignore_ascii_case("BaseCost ASC") || s.eq_ignore_ascii_case("BaseCost") => {
            query.order_by_asc(product::Column::BaseCost)
        }
        s if s.eq_ignore_ascii_case("SalesPrice DESC") => {
            query.order_by_desc(product::Column::SalesPrice)
        }
        s if s.eq_ignore_ascii_case("SalesPrice ASC") || s.eq_ignore_ascii_case("SalesPrice") => {
            query.order_by_asc(product::Column::SalesPrice)
        }
        s if s.eq_ignore_ascii_case("HSN DESC") => query.order_by_desc(product::Column::HsnCode),
        s if s.eq_ignore_ascii_case("HSN ASC") || s.eq_ignore_ascii_case("HSN") => {
            query.order_by_asc(product::Column::HsnCode)
        }
        _ => query,
    };

    let page = q.page.get();
    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();

    let currency = load_default_currency_format(db).await;
    let mut rows = Vec::with_capacity(models.len());
    for p in models {
        rows.push(ProductRow {
            id: p.id,
            product_type: p.product_type.as_str().to_string(),
            reference: p.reference.unwrap_or_default(),
            name: p.name,
            base_cost: currency.display(p.base_cost),
            sales_price: currency.display(p.sales_price),
            // Plain number for picker → rate fill (no currency symbol).
            sales_price_value: decimal::decimal_display(p.sales_price),
            hsn_code: p.hsn_code.to_string(),
        });
    }
    (rows, page, total)
}

pub async fn list(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ProductListQuery>,
) -> maud::Markup {
    let (rows, page, total) = query_products(&state.db, &q, &ctx, PAGE_SIZE).await;
    let products = ObjectList::from_page(rows, page, PAGE_SIZE, total);
    let page = ProductListPage {
        products,
        filter_name: q.name.clone().unwrap_or_default(),
        filter_reference: q.reference.clone().unwrap_or_default(),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<ProductTableKey>() {
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
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(p) = find_product_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-products/").into_response();
    };
    let tax_ids = load_product_tax_ids(&state.db, id).await;
    let taxes = load_taxes_by_ids(&state.db, &tax_ids)
        .await
        .unwrap_or_default();
    let tax_labels: Vec<String> = taxes.iter().map(tax_label).collect();
    let currency = load_default_currency_format(&state.db).await;
    let page = ProductDetailPage {
        id: p.id,
        name: p.name,
        product_type: p.product_type.as_str().to_string(),
        reference: p.reference.unwrap_or_default(),
        remarks: p.remarks.unwrap_or_default(),
        base_cost: currency.display(p.base_cost),
        sales_price: currency.display(p.sales_price),
        hsn_code: p.hsn_code.to_string(),
        taxes: tax_labels.join(", "),
        can_edit: require_superuser(&ctx),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    if !require_superuser(&ctx) {
        return maud::html! { div class="alert alert-error" { "Forbidden" } };
    }
    let default_tax_ids = load_default_product_tax_ids(&state.db).await;
    let tax_items = tax_items_from_ids(&state.db, &default_tax_ids).await;
    let page = ProductCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        name: String::new(),
        product_type: product::PRODUCT_TYPE_GOODS.to_string(),
        reference: String::new(),
        remarks: String::new(),
        base_cost: String::new(),
        sales_price: String::new(),
        hsn_code: 0,
        tax_items,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

async fn product_edit_modal_page_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ProductForm,
    id: i64,
    form_name: String,
    error: String,
) -> ProductEditModalPage {
    let tax_items = tax_items_from_ids(db, &form.tax_ids).await;
    ProductEditModalPage {
        id,
        form_name,
        name: form.name.clone(),
        product_type: form.product_type.clone(),
        reference: form.reference.clone(),
        remarks: form.remarks.clone(),
        base_cost: form.base_cost.clone(),
        sales_price: form.sales_price.clone(),
        hsn_code: form.hsn_code,
        tax_items,
        error,
    }
}

async fn product_create_modal_page_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ProductForm,
    form_name: String,
    refresh_table: String,
    error: String,
) -> ProductCreateModalPage {
    let tax_items = tax_items_from_ids(db, &form.tax_ids).await;
    ProductCreateModalPage {
        form_name,
        refresh_table,
        name: form.name.clone(),
        product_type: form.product_type.clone(),
        reference: form.reference.clone(),
        remarks: form.remarks.clone(),
        base_cost: form.base_cost.clone(),
        sales_price: form.sales_price.clone(),
        hsn_code: form.hsn_code,
        tax_items,
        error,
    }
}

async fn save_product_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ProductForm,
    id: Option<i64>,
) -> Result<i64, String> {
    let base_cost = parse_decimal(&form.base_cost).ok_or("invalid base cost")?;
    let sales_price = parse_decimal(&form.sales_price).ok_or("invalid sales price")?;
    let product_type = ProductType::parse(&form.product_type).ok_or("invalid product type")?;
    let now = Utc::now();
    let tax_ids = &form.tax_ids;

    if let Some(id) = id {
        let mut am: product::ActiveModel = ProductEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("product not found")?
            .into();
        am.name = Set(form.name.clone());
        am.product_type = Set(product_type);
        am.reference = Set(if form.reference.trim().is_empty() {
            None
        } else {
            Some(form.reference.trim().to_string())
        });
        am.remarks = Set(if form.remarks.is_empty() {
            None
        } else {
            Some(form.remarks.clone())
        });
        am.base_cost = Set(decimal::normalize(base_cost));
        am.sales_price = Set(decimal::normalize(sales_price));
        am.hsn_code = Set(form.hsn_code);
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        Ok(id)
    } else {
        let am = product::ActiveModel {
            name: Set(form.name.clone()),
            product_type: Set(product_type),
            reference: Set(if form.reference.trim().is_empty() {
                None
            } else {
                Some(form.reference.trim().to_string())
            }),
            remarks: Set(if form.remarks.is_empty() {
                None
            } else {
                Some(form.remarks.clone())
            }),
            base_cost: Set(decimal::normalize(base_cost)),
            sales_price: Set(decimal::normalize(sales_price)),
            hsn_code: Set(form.hsn_code),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let m = am.insert(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, m.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        Ok(m.id)
    }
}

pub async fn create_post(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ProductForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-products/").into_response();
    }
    match save_product_from_form(&state.db, &form, None).await {
        Ok(id) => respond_create_modal_done::<ProductCreateModalKey>(
            &htmx,
            &q.refresh_table(),
            &ProductDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page = product_create_modal_page_from_form(
                &state.db,
                &form,
                q.form_name(),
                q.refresh_table(),
                e,
            )
            .await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn edit_get(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-products/").into_response();
    }
    let Some(p) = find_product_scoped(&state.db, id, &ctx).await else {
        return Redirect::to("/finance-products/").into_response();
    };
    let tax_ids = load_product_tax_ids(&state.db, id).await;
    let tax_items = tax_items_from_ids(&state.db, &tax_ids).await;
    let page = ProductEditModalPage {
        id: p.id,
        form_name: q.form_name(),
        name: p.name,
        product_type: p.product_type.as_str().to_string(),
        reference: p.reference.unwrap_or_default(),
        remarks: p.remarks.unwrap_or_default(),
        base_cost: decimal::decimal_display(p.base_cost),
        sales_price: decimal::decimal_display(p.sales_price),
        hsn_code: p.hsn_code,
        tax_items,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ProductForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-products/").into_response();
    }
    match save_product_from_form(&state.db, &form, Some(id)).await {
        Ok(_) => respond_edit_modal_done::<ProductEditModalKey>(
            &htmx,
            &ProductDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page =
                product_edit_modal_page_from_form(&state.db, &form, id, q.form_name(), e).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_post(
    Cap(state): Cap<ProductsState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-products/").into_response();
    }
    if find_product_scoped(&state.db, id, &ctx).await.is_some() {
        let _ = product::Entity::delete_by_id(id).exec(&state.db).await;
    }
    Redirect::to("/finance-products/").into_response()
}

pub async fn select(
    Cap(state): Cap<ProductsState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ProductSelectQuery>,
) -> Response {
    let (rows, page, total) = query_products(&state.db, &q.filter, &ctx, PAGE_SIZE).await;
    let products = ObjectList::from_page(rows, page, PAGE_SIZE, total);
    let page = ProductSelectPage {
        products,
        filter_name: q.filter.name.clone().unwrap_or_default(),
        filter_reference: q.filter.reference.clone().unwrap_or_default(),
        target_input: q.target_input.unwrap_or_else(|| "ProductID".to_string()),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        can_edit: require_superuser(&ctx),
    };
    respond_picker_select::<ProductSelectTableKey, ProductSelectModalKey, _>(&htmx, &page)
        .into_response()
}
