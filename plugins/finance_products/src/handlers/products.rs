use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_plugin_finance_common::decimal::{self, dec_is_zero};
use lariv_plugin_inventory::entities::stock::{self as stock_entity, Entity as StockEntity};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseBackend, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder,
};

use lariv_core::components::FilterClauseView;
use lariv_core::list_filters::{
    FilterField, FilterRowError, apply_filters, parse_list_filters, text_column,
};
use serde::Deserialize;

use lariv_core::components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done_fk_extra, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::{
    entities::product::{self, Entity as ProductEntity},
    forms::ProductForm,
    handlers::ModalNameQuery,
    keys::{
        ProductCreateModalKey, ProductDeleteModalKey, ProductEditModalKey, ProductSelectModalKey,
        ProductSelectTableKey, ProductTableKey,
    },
    preferences::{load_default_product_tax_ids, load_product_tax_ids, set_product_tax_ids},
    pricing,
    routes::{ProductDefaultRouteTag, ProductDetailRouteTag},
    scope::find_product_scoped,
    state::ProductsState,
    templates::{
        ConfirmDeletePage, ProductCreateModalPage, ProductDetailPage, ProductEditModalPage,
        ProductListPage, ProductRow, ProductSelectPage,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct ProductListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

struct ProductListFilters {
    fields: Vec<FilterField>,
    filters: Vec<FilterClauseView>,
    filter_errors: Vec<FilterRowError>,
    condition: Condition,
}

fn product_list_filters(query: &str, backend: DatabaseBackend) -> ProductListFilters {
    let fields = vec![
        text_column("name", "Name", product::Column::Name),
        text_column("reference", "Reference", product::Column::Reference),
    ];
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    ProductListFilters {
        fields,
        filters,
        filter_errors: applied.errors,
        condition: applied.condition,
    }
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

async fn stock_names_by_id(db: &sea_orm::DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    if ids.is_empty() {
        return HashMap::new();
    }
    StockEntity::find()
        .filter(stock_entity::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|stock| (stock.id, stock.name))
        .collect()
}

async fn stock_display(db: &sea_orm::DatabaseConnection, stock_id: i64) -> String {
    if stock_id <= 0 {
        return String::new();
    }
    stock_names_by_id(db, &[stock_id])
        .await
        .remove(&stock_id)
        .unwrap_or_default()
}

async fn tax_items_from_ids(
    db: &sea_orm::DatabaseConnection,
    tax_ids: &[i64],
) -> Vec<ManyToManyItem> {
    load_taxes_by_ids(db, tax_ids)
        .await
        .unwrap_or_default()
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect()
}

async fn query_products(
    db: &sea_orm::DatabaseConnection,
    query_string: &str,
    sort: Option<&str>,
    page: u32,
    page_size: u32,
) -> (Vec<ProductRow>, u32, u64, ProductListFilters) {
    let filters = product_list_filters(query_string, db.get_database_backend());
    let mut query =
        scope_allowed::<super::super::routes::FinanceProductsView, _>(ProductEntity::find());
    query = query.filter(filters.condition.clone());

    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(product::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(product::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Stock DESC") => query.order_by_desc(product::Column::StockId),
        s if s.eq_ignore_ascii_case("Stock ASC") || s.eq_ignore_ascii_case("Stock") => {
            query.order_by_asc(product::Column::StockId)
        }
        s if s.eq_ignore_ascii_case("Reference DESC") => {
            query.order_by_desc(product::Column::Reference)
        }
        s if s.eq_ignore_ascii_case("Reference ASC") || s.eq_ignore_ascii_case("Reference") => {
            query.order_by_asc(product::Column::Reference)
        }
        s if s.eq_ignore_ascii_case("BasePriceFormula DESC") => {
            query.order_by_desc(product::Column::BasePriceFormula)
        }
        s if s.eq_ignore_ascii_case("BasePriceFormula ASC")
            || s.eq_ignore_ascii_case("BasePriceFormula") =>
        {
            query.order_by_asc(product::Column::BasePriceFormula)
        }
        s if s.eq_ignore_ascii_case("SalesPriceFormula DESC") => {
            query.order_by_desc(product::Column::SalesPriceFormula)
        }
        s if s.eq_ignore_ascii_case("SalesPriceFormula ASC")
            || s.eq_ignore_ascii_case("SalesPriceFormula") =>
        {
            query.order_by_asc(product::Column::SalesPriceFormula)
        }
        s if s.eq_ignore_ascii_case("HSN DESC") => query.order_by_desc(product::Column::HsnCode),
        s if s.eq_ignore_ascii_case("HSN ASC") || s.eq_ignore_ascii_case("HSN") => {
            query.order_by_asc(product::Column::HsnCode)
        }
        _ => query.order_by_desc(product::Column::Id),
    };

    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();

    let stock_ids: Vec<i64> = models.iter().map(|p| p.stock_id).collect();
    let stock_names = stock_names_by_id(db, &stock_ids).await;
    let mut rows = Vec::with_capacity(models.len());
    for p in models {
        let has_formula = pricing::has_variable_pricing(&p.variables);
        let sales_price_value = if has_formula {
            String::new()
        } else {
            pricing::unit_amount(&p.sales_price_formula)
                .ok()
                .filter(|amount| !dec_is_zero(*amount))
                .map(|amount| decimal::decimal_display(amount))
                .unwrap_or_default()
        };
        rows.push(ProductRow {
            id: p.id,
            stock_name: stock_names.get(&p.stock_id).cloned().unwrap_or_default(),
            reference: p.reference.unwrap_or_default(),
            name: p.name,
            base_price_formula: p.base_price_formula,
            sales_price_formula: p.sales_price_formula,
            sales_price_value,
            hsn_code: p.hsn_code.to_string(),
            has_formula,
            variables_json: pricing::variable_input_specs_json(&p.variables),
            remarks: p.remarks.unwrap_or_default(),
        });
    }
    (rows, page, total, filters)
}

pub async fn list(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ProductListQuery>,
) -> maud::Markup {
    let (rows, page_num, total, filters) = query_products(
        &state.db,
        uri.query().unwrap_or(""),
        q.sort.as_deref(),
        q.page.get(),
        q.page_size.get(),
    )
    .await;
    let products = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = ProductListPage {
        products,
        fields: filters.fields,
        filters: filters.filters,
        filter_errors: filters.filter_errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
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
    let Some(p) = find_product_scoped(&state.db, id).await else {
        return Redirect::to(&ProductDefaultRouteTag.url()).into_response();
    };
    let tax_ids = load_product_tax_ids(&state.db, id).await;
    let taxes = load_taxes_by_ids(&state.db, &tax_ids)
        .await
        .unwrap_or_default();
    let tax_labels: Vec<String> = taxes.iter().map(tax_label).collect();
    let template = journal_template_summary(&state.db, p.id).await;
    let page = ProductDetailPage {
        id: p.id,
        name: p.name,
        stock_name: stock_display(&state.db, p.stock_id).await,
        reference: p.reference.unwrap_or_default(),
        remarks: p.remarks.unwrap_or_default(),
        hsn_code: p.hsn_code.to_string(),
        taxes: tax_labels.join(", "),
        variables: pricing::schema_entries_from_stored(&p.variables).join("\n"),
        base_price_formula: p.base_price_formula,
        sales_price_formula: p.sales_price_formula,
        journal_name: template.journal_name,
        journal_lines: template.lines_display,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_get(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let (journal_id, journal_display) =
        lariv_plugin_finance_accounts::scope::load_default_journal_form(&state.db).await;
    let default_tax_ids = load_default_product_tax_ids(&state.db).await;
    let tax_items = tax_items_from_ids(&state.db, &default_tax_ids).await;
    let page = ProductCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        stock_id: 0,
        stock_display: String::new(),
        reference: String::new(),
        remarks: String::new(),
        variables: Vec::new(),
        base_price_formula: String::new(),
        sales_price_formula: String::new(),
        journal_id,
        journal_display,
        journal_lines: crate::journal_template::empty_lines_json(),
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
        stock_id: form.stock_id,
        stock_display: stock_display(db, form.stock_id).await,
        reference: form.reference.clone(),
        remarks: form.remarks.clone(),
        variables: form.variables.clone(),
        base_price_formula: form.base_price_formula.clone(),
        sales_price_formula: form.sales_price_formula.clone(),
        journal_id: form.journal_id.clone(),
        journal_display: journal_display(db, &form.journal_id).await,
        journal_lines: form.journal_lines.clone(),
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
    target_input: String,
    error: String,
) -> ProductCreateModalPage {
    let tax_items = tax_items_from_ids(db, &form.tax_ids).await;
    ProductCreateModalPage {
        form_name,
        refresh_table,
        target_input,
        name: form.name.clone(),
        stock_id: form.stock_id,
        stock_display: stock_display(db, form.stock_id).await,
        reference: form.reference.clone(),
        remarks: form.remarks.clone(),
        variables: form.variables.clone(),
        base_price_formula: form.base_price_formula.clone(),
        sales_price_formula: form.sales_price_formula.clone(),
        journal_id: form.journal_id.clone(),
        journal_display: journal_display(db, &form.journal_id).await,
        journal_lines: form.journal_lines.clone(),
        hsn_code: form.hsn_code,
        tax_items,
        error,
    }
}

struct TemplateFormFields {
    journal_id: String,
    journal_display: String,
    journal_lines: String,
}

struct TemplateSummary {
    journal_name: String,
    lines_display: String,
}

async fn journal_display(db: &sea_orm::DatabaseConnection, raw: &str) -> String {
    let id = raw.trim().parse::<i64>().ok().filter(|id| *id > 0);
    let label = lariv_plugin_finance_accounts::scope::load_journal_display_label(db, id).await;
    if label == "—" { String::new() } else { label }
}

async fn template_form_fields(
    db: &sea_orm::DatabaseConnection,
    product_id: i64,
) -> TemplateFormFields {
    let Ok(Some(template)) = crate::journal_template::load_template(db, product_id).await else {
        let (journal_id, journal_display) =
            lariv_plugin_finance_accounts::scope::load_default_journal_form(db).await;
        return TemplateFormFields {
            journal_id,
            journal_display,
            journal_lines: crate::journal_template::empty_lines_json(),
        };
    };
    let items = &template.items;
    let mut labels = Vec::with_capacity(items.len());
    for item in items {
        let label = lariv_plugin_finance_accounts::scope::load_account_parent_label(
            db,
            Some(item.account_id),
        )
        .await;
        labels.push(if label == "—" { String::new() } else { label });
    }
    TemplateFormFields {
        journal_id: template.journal_id.to_string(),
        journal_display: journal_display(db, &template.journal_id.to_string()).await,
        journal_lines: crate::journal_template::lines_to_json(items, &labels),
    }
}

async fn journal_template_summary(
    db: &sea_orm::DatabaseConnection,
    product_id: i64,
) -> TemplateSummary {
    let Ok(Some(template)) = crate::journal_template::load_template(db, product_id).await else {
        return TemplateSummary {
            journal_name: "—".into(),
            lines_display: "—".into(),
        };
    };
    let items = &template.items;
    let journal_name = lariv_plugin_finance_accounts::scope::load_journal_display_label(
        db,
        Some(template.journal_id),
    )
    .await;
    let mut lines = Vec::new();
    for item in items {
        let account = lariv_plugin_finance_accounts::scope::load_account_parent_label(
            db,
            Some(item.account_id),
        )
        .await;
        lines.push(format!("{account}: {}", item.formula));
    }
    TemplateSummary {
        journal_name,
        lines_display: if lines.is_empty() {
            "—".into()
        } else {
            lines.join("\n")
        },
    }
}

fn optional_text(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

async fn save_product_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ProductForm,
    id: Option<i64>,
) -> Result<i64, String> {
    let (variables, schema) = pricing::store_schema(&form.variables)?;
    let base_price_formula = form.base_price_formula.trim().to_string();
    let sales_price_formula = form.sales_price_formula.trim().to_string();
    pricing::validate_price_formula(&schema, &base_price_formula, "base price formula")?;
    pricing::validate_price_formula(&schema, &sales_price_formula, "sales price formula")?;
    crate::journal_template::ensure_schema_allows_bindings(&schema)?;
    if form.stock_id <= 0 {
        return Err("select a stock".to_string());
    }
    if StockEntity::find_by_id(form.stock_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err("stock not found".to_string());
    }
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
        am.stock_id = Set(form.stock_id);
        am.reference = Set(if form.reference.trim().is_empty() {
            None
        } else {
            Some(form.reference.trim().to_string())
        });
        am.remarks = Set(optional_text(&form.remarks));
        am.variables = Set(variables);
        am.base_price_formula = Set(base_price_formula);
        am.sales_price_formula = Set(sales_price_formula);
        am.hsn_code = Set(form.hsn_code);
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        crate::journal_template::save_product_template(
            db,
            id,
            &form.journal_id,
            &form.journal_lines,
            &schema,
        )
        .await?;
        Ok(id)
    } else {
        let am = product::ActiveModel {
            name: Set(form.name.clone()),
            stock_id: Set(form.stock_id),
            reference: Set(if form.reference.trim().is_empty() {
                None
            } else {
                Some(form.reference.trim().to_string())
            }),
            remarks: Set(optional_text(&form.remarks)),
            variables: Set(variables),
            base_price_formula: Set(base_price_formula),
            sales_price_formula: Set(sales_price_formula),
            hsn_code: Set(form.hsn_code),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let m = am.insert(db).await.map_err(|e| e.to_string())?;
        set_product_tax_ids(db, m.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        crate::journal_template::save_product_template(
            db,
            m.id,
            &form.journal_id,
            &form.journal_lines,
            &schema,
        )
        .await?;
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
    match save_product_from_form(&state.db, &form, None).await {
        Ok(id) => {
            let stored = pricing::store_schema(&form.variables)
                .map(|(json, _)| json)
                .unwrap_or_else(|_| "{}".into());
            let variables_json = pricing::variable_input_specs_json(&stored);
            let has_formula = pricing::has_variable_pricing(&stored);
            let sales_price = if has_formula {
                String::new()
            } else {
                pricing::unit_amount(&form.sales_price_formula)
                    .map(|amount| decimal::decimal_display(amount))
                    .unwrap_or_default()
            };
            respond_create_modal_done_fk_extra::<ProductCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &ProductDetailRouteTag::new(id).url(),
                id,
                &form.name,
                &q.target_input(),
                &[
                    ("sales_price", sales_price.as_str()),
                    ("has_formula", if has_formula { "1" } else { "0" }),
                    ("variables", variables_json.as_str()),
                    ("remarks", form.remarks.as_str()),
                ],
            )
        }
        Err(e) => {
            let page = product_create_modal_page_from_form(
                &state.db,
                &form,
                q.form_name(),
                q.refresh_table(),
                q.target_input(),
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
    let Some(p) = find_product_scoped(&state.db, id).await else {
        return Redirect::to(&ProductDefaultRouteTag.url()).into_response();
    };
    let tax_ids = load_product_tax_ids(&state.db, id).await;
    let tax_items = tax_items_from_ids(&state.db, &tax_ids).await;
    let template = template_form_fields(&state.db, p.id).await;
    let page = ProductEditModalPage {
        id: p.id,
        form_name: q.form_name(),
        name: p.name,
        stock_id: p.stock_id,
        stock_display: stock_display(&state.db, p.stock_id).await,
        reference: p.reference.unwrap_or_default(),
        remarks: p.remarks.unwrap_or_default(),
        variables: pricing::schema_entries_from_stored(&p.variables),
        base_price_formula: p.base_price_formula,
        sales_price_formula: p.sales_price_formula,
        journal_id: template.journal_id,
        journal_display: template.journal_display,
        journal_lines: template.journal_lines,
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

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        modal_uid: ProductDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this product?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_products.ProductDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<ProductsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_product_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&ProductDefaultRouteTag.url()).into_response();
    }
    match ProductEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&ProductDefaultRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete product");
            let page = ConfirmDeletePage {
                modal_uid: ProductDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this product?".into(),
                form_name: "p_finance_products.ProductDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<ProductsState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ProductSelectQuery>,
) -> Response {
    let (rows, page_num, total, filters) = query_products(
        &state.db,
        uri.query().unwrap_or(""),
        q.filter.sort.as_deref(),
        q.filter.page.get(),
        q.filter.page_size.get(),
    )
    .await;
    let products = ObjectList::from_page(rows, page_num, q.filter.page_size.get(), total);
    let page = ProductSelectPage {
        products,
        fields: filters.fields,
        filters: filters.filters,
        filter_errors: filters.filter_errors,
        target_input: q.target_input.unwrap_or_else(|| "ProductID".to_string()),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select::<ProductSelectTableKey, ProductSelectModalKey, _>(&htmx, &page)
        .into_response()
}
