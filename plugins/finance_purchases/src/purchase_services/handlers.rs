use crate::routes::{PurchaseServiceDefaultRouteTag, PurchaseServiceDetailRouteTag};
use crate::state::PurchasesState;
use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use lariv_core::web::ModalFormQuery as ModalNameQuery;
use lariv_plugin_finance_common::decimal::{self, dec_is_zero};
use lariv_plugin_users::role_authorization::scope_allowed;
use sea_orm::sea_query::Condition;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseBackend, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder,
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

use crate::purchase_services::{
    entities::service::{self, Entity as ServiceEntity},
    forms::ServiceForm,
    keys::{
        ServiceCreateModalKey, ServiceDeleteModalKey, ServiceEditModalKey, ServiceSelectModalKey,
        ServiceSelectTableKey, ServiceTableKey,
    },
    preferences::{load_default_service_tax_ids, load_service_tax_ids, set_service_tax_ids},
    pricing,
    scope::find_service_scoped,
    templates::{
        ConfirmDeletePage, ServiceCreateModalPage, ServiceDetailPage, ServiceEditModalPage,
        ServiceListPage, ServiceRow, ServiceSelectPage,
    },
};

#[derive(Debug, Deserialize, Default)]
pub struct ServiceListQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

struct ServiceListFilters {
    fields: Vec<FilterField>,
    filters: Vec<FilterClauseView>,
    filter_errors: Vec<FilterRowError>,
    condition: Condition,
}

fn service_list_filters(query: &str, backend: DatabaseBackend) -> ServiceListFilters {
    let fields = vec![
        text_column("name", "Name", service::Column::Name),
        text_column("reference", "Reference", service::Column::Reference),
    ];
    let clauses = parse_list_filters(query);
    let applied = apply_filters(backend, &fields, &clauses);
    let filters = FilterClauseView::from_clauses(&clauses, &applied.errors);
    ServiceListFilters {
        fields,
        filters,
        filter_errors: applied.errors,
        condition: applied.condition,
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct ServiceSelectQuery {
    #[serde(flatten)]
    pub filter: ServiceListQuery,
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
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect()
}

async fn query_services(
    db: &sea_orm::DatabaseConnection,
    query_string: &str,
    sort: Option<&str>,
    page: u32,
    page_size: u32,
) -> (Vec<ServiceRow>, u32, u64, ServiceListFilters) {
    let filters = service_list_filters(query_string, db.get_database_backend());
    let mut query = scope_allowed::<crate::routes::FinancePurchasesView, _>(ServiceEntity::find());
    query = query.filter(filters.condition.clone());

    let sort = sort.unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("Name DESC") => query.order_by_desc(service::Column::Name),
        s if s.eq_ignore_ascii_case("Name ASC") || s.eq_ignore_ascii_case("Name") => {
            query.order_by_asc(service::Column::Name)
        }
        s if s.eq_ignore_ascii_case("Reference DESC") => {
            query.order_by_desc(service::Column::Reference)
        }
        s if s.eq_ignore_ascii_case("Reference ASC") || s.eq_ignore_ascii_case("Reference") => {
            query.order_by_asc(service::Column::Reference)
        }
        s if s.eq_ignore_ascii_case("BasePriceFormula DESC") => {
            query.order_by_desc(service::Column::BasePriceFormula)
        }
        s if s.eq_ignore_ascii_case("BasePriceFormula ASC")
            || s.eq_ignore_ascii_case("BasePriceFormula") =>
        {
            query.order_by_asc(service::Column::BasePriceFormula)
        }
        s if s.eq_ignore_ascii_case("SalesPriceFormula DESC") => {
            query.order_by_desc(service::Column::SalesPriceFormula)
        }
        s if s.eq_ignore_ascii_case("SalesPriceFormula ASC")
            || s.eq_ignore_ascii_case("SalesPriceFormula") =>
        {
            query.order_by_asc(service::Column::SalesPriceFormula)
        }
        s if s.eq_ignore_ascii_case("SAC DESC") => query.order_by_desc(service::Column::SacCode),
        s if s.eq_ignore_ascii_case("SAC ASC") || s.eq_ignore_ascii_case("SAC") => {
            query.order_by_asc(service::Column::SacCode)
        }
        _ => query.order_by_desc(service::Column::Id),
    };

    let paginator = query.paginate(db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page as u64).saturating_sub(1))
        .await
        .unwrap_or_default();

    let mut rows = Vec::with_capacity(models.len());
    for p in models {
        let has_formula = pricing::has_variable_pricing(&p.variables);
        let sales_price_value = if has_formula {
            String::new()
        } else {
            pricing::unit_amount(&p.sales_price_formula)
                .ok()
                .filter(|amount| !dec_is_zero(*amount))
                .map(decimal::decimal_display)
                .unwrap_or_default()
        };
        rows.push(ServiceRow {
            id: p.id,
            reference: p.reference.unwrap_or_default(),
            name: p.name,
            base_price_formula: p.base_price_formula,
            sales_price_formula: p.sales_price_formula,
            sales_price_value,
            sac_code: p.sac_code.to_string(),
            has_formula,
            variables_json: pricing::variable_input_specs_json(&p.variables),
            remarks: p.remarks.unwrap_or_default(),
        });
    }
    (rows, page, total, filters)
}

pub async fn list(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ServiceListQuery>,
) -> maud::Markup {
    let (rows, page_num, total, filters) = query_services(
        &state.db,
        uri.query().unwrap_or(""),
        q.sort.as_deref(),
        q.page.get(),
        q.page_size.get(),
    )
    .await;
    let services = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = ServiceListPage {
        services,
        fields: filters.fields,
        filters: filters.filters,
        filter_errors: filters.filter_errors,
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<ServiceTableKey>() {
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
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(p) = find_service_scoped(&state.db, id).await else {
        return Redirect::to(&PurchaseServiceDefaultRouteTag.url()).into_response();
    };
    let tax_ids = load_service_tax_ids(&state.db, id).await;
    let taxes = load_taxes_by_ids(&state.db, &tax_ids)
        .await
        .unwrap_or_default();
    let tax_labels: Vec<String> = taxes.iter().map(tax_label).collect();
    let template = journal_template_summary(&state.db, p.id).await;
    let page = ServiceDetailPage {
        id: p.id,
        name: p.name,
        reference: p.reference.unwrap_or_default(),
        remarks: p.remarks.unwrap_or_default(),
        sac_code: p.sac_code.to_string(),
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
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> maud::Markup {
    let defaults = crate::purchase_services::journal_template::preference_form_fields(
        &state.db,
        "template-line",
    )
    .await;
    let default_tax_ids = load_default_service_tax_ids(&state.db).await;
    let tax_items = tax_items_from_ids(&state.db, &default_tax_ids).await;
    let page = ServiceCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        name: String::new(),
        reference: String::new(),
        remarks: String::new(),
        variables: Vec::new(),
        base_price_formula: String::new(),
        sales_price_formula: String::new(),
        journal_id: defaults.journal_id,
        journal_display: defaults.journal_display,
        journal_lines: defaults.journal_lines,
        sac_code: 0,
        tax_items,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

async fn service_edit_modal_page_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ServiceForm,
    id: i64,
    form_name: String,
    error: String,
) -> ServiceEditModalPage {
    let tax_items = tax_items_from_ids(db, &form.tax_ids).await;
    ServiceEditModalPage {
        id,
        form_name,
        name: form.name.clone(),
        reference: form.reference.clone(),
        remarks: form.remarks.clone(),
        variables: form.variables.clone(),
        base_price_formula: form.base_price_formula.clone(),
        sales_price_formula: form.sales_price_formula.clone(),
        journal_id: form.journal_id.clone(),
        journal_display: journal_display(db, &form.journal_id).await,
        journal_lines: form.journal_lines.clone(),
        sac_code: form.sac_code,
        tax_items,
        error,
    }
}

async fn service_create_modal_page_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ServiceForm,
    form_name: String,
    refresh_table: String,
    target_input: String,
    error: String,
) -> ServiceCreateModalPage {
    let tax_items = tax_items_from_ids(db, &form.tax_ids).await;
    ServiceCreateModalPage {
        form_name,
        refresh_table,
        target_input,
        name: form.name.clone(),
        reference: form.reference.clone(),
        remarks: form.remarks.clone(),
        variables: form.variables.clone(),
        base_price_formula: form.base_price_formula.clone(),
        sales_price_formula: form.sales_price_formula.clone(),
        journal_id: form.journal_id.clone(),
        journal_display: journal_display(db, &form.journal_id).await,
        journal_lines: form.journal_lines.clone(),
        sac_code: form.sac_code,
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
    service_id: i64,
) -> TemplateFormFields {
    let Ok(Some(template)) =
        crate::purchase_services::journal_template::load_template(db, service_id).await
    else {
        let (journal_id, journal_display) =
            lariv_plugin_finance_accounts::scope::load_default_journal_form(db).await;
        return TemplateFormFields {
            journal_id,
            journal_display,
            journal_lines: crate::purchase_services::journal_template::empty_lines_json(),
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
        journal_lines: crate::purchase_services::journal_template::lines_to_json(items, &labels),
    }
}

async fn journal_template_summary(
    db: &sea_orm::DatabaseConnection,
    service_id: i64,
) -> TemplateSummary {
    let Ok(Some(template)) =
        crate::purchase_services::journal_template::load_template(db, service_id).await
    else {
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

async fn save_service_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &ServiceForm,
    id: Option<i64>,
) -> Result<i64, String> {
    let (variables, schema) = pricing::store_schema(&form.variables)?;
    let base_price_formula = form.base_price_formula.trim().to_string();
    let sales_price_formula = form.sales_price_formula.trim().to_string();
    pricing::validate_price_formula(&schema, &base_price_formula, "base price formula")?;
    pricing::validate_price_formula(&schema, &sales_price_formula, "sales price formula")?;
    crate::purchase_services::journal_template::ensure_schema_allows_bindings(&schema)?;
    let now = Utc::now();
    let tax_ids = &form.tax_ids;

    if let Some(id) = id {
        let mut am: service::ActiveModel = ServiceEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("service not found")?
            .into();
        am.name = Set(form.name.clone());
        am.reference = Set(if form.reference.trim().is_empty() {
            None
        } else {
            Some(form.reference.trim().to_string())
        });
        am.remarks = Set(optional_text(&form.remarks));
        am.variables = Set(variables);
        am.base_price_formula = Set(base_price_formula);
        am.sales_price_formula = Set(sales_price_formula);
        am.sac_code = Set(form.sac_code);
        am.updated_at = Set(Some(now));
        am.update(db).await.map_err(|e| e.to_string())?;
        set_service_tax_ids(db, id, tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        crate::purchase_services::journal_template::save_service_template(
            db,
            id,
            &form.journal_id,
            &form.journal_lines,
            &schema,
        )
        .await?;
        Ok(id)
    } else {
        let am = service::ActiveModel {
            name: Set(form.name.clone()),
            reference: Set(if form.reference.trim().is_empty() {
                None
            } else {
                Some(form.reference.trim().to_string())
            }),
            remarks: Set(optional_text(&form.remarks)),
            variables: Set(variables),
            base_price_formula: Set(base_price_formula),
            sales_price_formula: Set(sales_price_formula),
            sac_code: Set(form.sac_code),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let m = am.insert(db).await.map_err(|e| e.to_string())?;
        set_service_tax_ids(db, m.id, tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        crate::purchase_services::journal_template::save_service_template(
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
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ServiceForm>,
) -> Response {
    match save_service_from_form(&state.db, &form, None).await {
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
                    .map(decimal::decimal_display)
                    .unwrap_or_default()
            };
            respond_create_modal_done_fk_extra::<ServiceCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &PurchaseServiceDetailRouteTag::new(id).url(),
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
            let page = service_create_modal_page_from_form(
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
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(p) = find_service_scoped(&state.db, id).await else {
        return Redirect::to(&PurchaseServiceDefaultRouteTag.url()).into_response();
    };
    let tax_ids = load_service_tax_ids(&state.db, id).await;
    let tax_items = tax_items_from_ids(&state.db, &tax_ids).await;
    let template = template_form_fields(&state.db, p.id).await;
    let page = ServiceEditModalPage {
        id: p.id,
        form_name: q.form_name(),
        name: p.name,
        reference: p.reference.unwrap_or_default(),
        remarks: p.remarks.unwrap_or_default(),
        variables: pricing::schema_entries_from_stored(&p.variables),
        base_price_formula: p.base_price_formula,
        sales_price_formula: p.sales_price_formula,
        journal_id: template.journal_id,
        journal_display: template.journal_display,
        journal_lines: template.journal_lines,
        sac_code: p.sac_code,
        tax_items,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    HtmlFormBody(form): HtmlFormBody<ServiceForm>,
) -> Response {
    match save_service_from_form(&state.db, &form, Some(id)).await {
        Ok(_) => respond_edit_modal_done::<ServiceEditModalKey>(
            &htmx,
            &PurchaseServiceDetailRouteTag::new(id).url(),
        ),
        Err(e) => {
            let page =
                service_edit_modal_page_from_form(&state.db, &form, id, q.form_name(), e).await;
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
        modal_uid: ServiceDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this service?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_purchases.PurchaseServiceDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_service_scoped(&state.db, id).await.is_none() {
        return Redirect::to(&PurchaseServiceDefaultRouteTag.url()).into_response();
    }
    match ServiceEntity::delete_by_id(id).exec(&state.db).await {
        Ok(_) => htmx.redirect(&PurchaseServiceDefaultRouteTag.url()),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete service");
            let page = ConfirmDeletePage {
                modal_uid: ServiceDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this service?".into(),
                form_name: "p_finance_purchases.PurchaseServiceDeleteForm".into(),
                id,
                error: e.to_string(),
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn select(
    Cap(state): Cap<PurchasesState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ServiceSelectQuery>,
) -> Response {
    let (rows, page_num, total, filters) = query_services(
        &state.db,
        uri.query().unwrap_or(""),
        q.filter.sort.as_deref(),
        q.filter.page.get(),
        q.filter.page_size.get(),
    )
    .await;
    let services = ObjectList::from_page(rows, page_num, q.filter.page_size.get(), total);
    let page = ServiceSelectPage {
        services,
        fields: filters.fields,
        filters: filters.filters,
        filter_errors: filters.filter_errors,
        target_input: q.target_input.unwrap_or_else(|| "ServiceID".to_string()),
        sort: q.filter.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.filter.page_size.get(),
    };
    respond_picker_select::<ServiceSelectTableKey, ServiceSelectModalKey, _>(&htmx, &page)
        .into_response()
}
