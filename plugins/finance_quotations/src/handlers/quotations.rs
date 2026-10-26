use axum::{
    Json,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{NaiveTime, Utc};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::Deserialize;

use lariv_core::components::{ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::HtmlFormBody;
use lariv_core::http::Cap;
use lariv_core::template::RenderAppPane;
use lariv_core::web::{
    Htmx, ModalFormQuery, QueryPage, QueryPageSize, html_built_page_or_app_layout,
    html_built_page_with_slots, respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_crm::entities::customer::Entity as CustomerEntity;
use lariv_plugin_crm::logic::customer::{detail_href, display_name};
use lariv_plugin_finance_accounts::scope::load_default_currency_format;
use lariv_plugin_finance_common::decimal::decimal_display;
use lariv_plugin_finance_invoices::logic::draft::{
    CreateDraftInput, DraftLinePending, DraftServiceLinePending, create_draft_invoice,
};
use lariv_plugin_finance_invoices::logic::draft_payment_term::{
    default_payment_term_lines_json, parse_payment_term_lines_json,
};
use lariv_plugin_finance_invoices::logic::invoice_goods_line_editor::invoice_goods_line_editor_preview_json;
use lariv_plugin_finance_invoices::logic::invoice_service_line_editor::invoice_service_line_editor_preview_json;
use lariv_plugin_finance_invoices::routes::DraftInvoiceDetailRouteTag;
use lariv_plugin_finance_products::entities::product::Entity as ProductEntity;
use lariv_plugin_finance_products::pricing as product_pricing;
use lariv_plugin_finance_services::entities::service::Entity as ServiceEntity;
use lariv_plugin_finance_services::pricing as service_pricing;
use lariv_plugin_users::middleware::RequireAuth;

use crate::entities::quotation::{self, Entity as QuotationEntity};
use crate::entities::{
    quotation_goods_line::{self, Entity as GoodsLineEntity},
    quotation_service_line::{self, Entity as ServiceLineEntity},
};
use crate::forms::QuotationForm;
use crate::keys::QuotationCreateModalKey;
use crate::logic::document::{
    SaveQuotation, copy_quotation, create_quotation, delete_quotation, parse_goods_lines,
    parse_quotation_date, parse_service_lines, update_quotation,
};
use crate::logic::editor::{
    goods_display_rows, goods_form_json, quotation_grand_total, service_display_rows,
    service_form_json,
};
use crate::logic::prefs::load_preferences;
use crate::logic::tax::{load_goods_line_tax_ids, load_service_line_tax_ids};
use crate::routes::{FinanceQuotationsMutate, QuotationDefaultRouteTag, QuotationDetailRouteTag};
use crate::state::QuotationsState;
use crate::templates::{
    ConfirmDeletePage, ConfirmInvoicePage, QuotationCreateModalPage, QuotationDetailPage,
    QuotationEditModalPage, QuotationFormFields, QuotationListPage, QuotationRow,
};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
}

#[derive(Debug, Deserialize)]
pub struct LinePriceQuery {
    product_id: i64,
    #[serde(default)]
    values: String,
}

#[derive(Debug, Deserialize)]
pub struct ServiceLinePriceQuery {
    service_id: i64,
    #[serde(default)]
    values: String,
}

pub async fn list(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<ListQuery>,
) -> maud::Markup {
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let paginator = QuotationEntity::find()
        .order_by_desc(quotation::Column::Date)
        .order_by_desc(quotation::Column::Id)
        .paginate(&state.db, u64::from(page_size));
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page(u64::from(page_num.saturating_sub(1)))
        .await
        .unwrap_or_default();
    let currency = load_default_currency_format(&state.db).await;
    let mut rows = Vec::with_capacity(models.len());
    for quotation in models {
        let customer = display_name(&state.db, quotation.customer_id).await;
        let total_amount = quotation_grand_total(&state.db, quotation.id).await;
        rows.push(QuotationRow {
            id: quotation.id,
            number: quotation.number,
            date: lariv_core::datetime::format_date(quotation.date),
            customer,
            total: currency.display(total_amount),
        });
    }
    let page = QuotationListPage {
        quotations: ObjectList::from_page(rows, page_num, page_size, total),
        path_and_query: uri
            .path_and_query()
            .map(|pq| pq.as_str().to_string())
            .unwrap_or_else(|| QuotationDefaultRouteTag.url()),
        page_size,
    };
    let slot_ctx = SlotCtx::from_auth(&ctx);
    if htmx.targets::<crate::keys::QuotationTableKey>() {
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

pub async fn goods_line_price(
    Cap(state): Cap<QuotationsState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<LinePriceQuery>,
) -> Json<serde_json::Value> {
    let raw = if q.values.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&q.values).unwrap_or_else(|_| serde_json::json!({}))
    };
    let Some(product) = ProductEntity::find_by_id(q.product_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Json(serde_json::json!({"pre_tax": "", "error": "unknown product"}));
    };
    if !product_pricing::has_variable_pricing(&product.variables) {
        return Json(serde_json::json!({"pre_tax": "", "error": ""}));
    }
    match product_pricing::price_line(&product.variables, &product.sales_price_formula, &raw) {
        Ok(priced) => Json(serde_json::json!({
            "pre_tax": decimal_display(priced.pre_tax),
            "error": "",
        })),
        Err(error) => Json(serde_json::json!({"pre_tax": "", "error": error})),
    }
}

pub async fn service_line_price(
    Cap(state): Cap<QuotationsState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<ServiceLinePriceQuery>,
) -> Json<serde_json::Value> {
    let raw = if q.values.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&q.values).unwrap_or_else(|_| serde_json::json!({}))
    };
    let Some(service) = ServiceEntity::find_by_id(q.service_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Json(serde_json::json!({"pre_tax": "", "error": "unknown service"}));
    };
    if !service_pricing::has_variable_pricing(&service.variables) {
        return Json(serde_json::json!({"pre_tax": "", "error": ""}));
    }
    match service_pricing::price_line(&service.variables, &service.sales_price_formula, &raw) {
        Ok(priced) => Json(serde_json::json!({
            "pre_tax": decimal_display(priced.pre_tax),
            "error": "",
        })),
        Err(error) => Json(serde_json::json!({"pre_tax": "", "error": error})),
    }
}

pub async fn detail(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(quotation) = QuotationEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to(&QuotationDefaultRouteTag.url()).into_response();
    };
    let customer = display_name(&state.db, quotation.customer_id).await;
    let customer_href = CustomerEntity::find_by_id(quotation.customer_id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .map(|row| detail_href(&row))
        .unwrap_or_default();
    let goods = goods_display_rows(&state.db, id).await;
    let services = service_display_rows(&state.db, id).await;
    let grand = quotation_grand_total(&state.db, id).await;
    let actions = crate::detail_actions::render_quotation_detail_actions(&state.db, id).await;
    let page = QuotationDetailPage {
        id,
        number: quotation.number.clone(),
        date: lariv_core::datetime::format_date(quotation.date),
        customer,
        customer_href,
        bank_account: quotation.bank_account.unwrap_or_default(),
        remarks: quotation.remarks,
        goods,
        services,
        grand_total: load_default_currency_format(&state.db).await.display(grand),
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<FinanceQuotationsMutate>(),
        ),
        actions_html: actions.into_string(),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

async fn empty_form_view(db: &sea_orm::DatabaseConnection) -> QuotationFormFields {
    let prefs = load_preferences(db).await;
    QuotationFormFields {
        number: String::new(),
        date: lariv_core::datetime::format_date(Utc::now().date_naive()),
        customer_id: String::new(),
        customer_display: String::new(),
        bank_account: prefs.default_bank_account.unwrap_or_default(),
        remarks: String::new(),
        goods_json: goods_form_json(db, 0).await,
        service_json: service_form_json(db, 0).await,
        goods_preview: invoice_goods_line_editor_preview_json(db).await,
        service_preview: invoice_service_line_editor_preview_json(db).await,
    }
}

async fn form_view_from_form(
    db: &sea_orm::DatabaseConnection,
    form: &QuotationForm,
) -> QuotationFormFields {
    let customer_display = if form.customer_id > 0 {
        display_name(db, form.customer_id).await
    } else {
        String::new()
    };
    QuotationFormFields {
        number: form.number.clone(),
        date: form.date.clone(),
        customer_id: if form.customer_id > 0 {
            form.customer_id.to_string()
        } else {
            String::new()
        },
        customer_display,
        bank_account: form.bank_account.clone(),
        remarks: form.remarks.clone(),
        goods_json: form.goods_lines_json.clone(),
        service_json: form.service_lines_json.clone(),
        goods_preview: invoice_goods_line_editor_preview_json(db).await,
        service_preview: invoice_service_line_editor_preview_json(db).await,
    }
}

fn save_input(form: &QuotationForm) -> Result<SaveQuotation, String> {
    Ok(SaveQuotation {
        existing_id: None,
        number: form.number.clone(),
        date: parse_quotation_date(&form.date)?,
        customer_id: form.customer_id,
        bank_account: form.bank_account.clone(),
        remarks: form.remarks.clone(),
        goods: parse_goods_lines(&form.goods_lines_json)?,
        services: parse_service_lines(&form.service_lines_json)?,
    })
}

pub async fn create_get(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let fields = empty_form_view(&state.db).await;
    let page = QuotationCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        fields,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn create_post(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<QuotationForm>,
) -> Response {
    match save_input(&form) {
        Ok(input) => match create_quotation(&state.db, input).await {
            Ok(saved) => respond_create_modal_done::<QuotationCreateModalKey>(
                &htmx,
                &q.refresh_table(),
                &QuotationDetailRouteTag::new(saved.id).url(),
            ),
            Err(error) => create_error_page(&state, &chrome, &ctx, &q, &form, error).await,
        },
        Err(error) => create_error_page(&state, &chrome, &ctx, &q, &form, error).await,
    }
}

pub async fn edit_get(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalFormQuery>,
) -> Response {
    let Some(quotation) = QuotationEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to(&QuotationDefaultRouteTag.url()).into_response();
    };
    let customer_display = display_name(&state.db, quotation.customer_id).await;
    let fields = QuotationFormFields {
        number: quotation.number.clone(),
        date: lariv_core::datetime::format_date(quotation.date),
        customer_id: quotation.customer_id.to_string(),
        customer_display,
        bank_account: quotation.bank_account.clone().unwrap_or_default(),
        remarks: quotation.remarks.clone(),
        goods_json: goods_form_json(&state.db, id).await,
        service_json: service_form_json(&state.db, id).await,
        goods_preview: invoice_goods_line_editor_preview_json(&state.db).await,
        service_preview: invoice_service_line_editor_preview_json(&state.db).await,
    };
    let page = QuotationEditModalPage {
        id,
        form_name: q.form_name(),
        number_label: quotation.number,
        fields,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalFormQuery>,
    HtmlFormBody(form): HtmlFormBody<QuotationForm>,
) -> Response {
    let input = match save_input(&form) {
        Ok(input) => input,
        Err(error) => {
            let fields = form_view_from_form(&state.db, &form).await;
            let page = QuotationEditModalPage {
                id,
                form_name: q.form_name(),
                number_label: form.number.clone(),
                fields,
                error,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    match update_quotation(&state.db, id, input).await {
        Ok(_) => respond_edit_modal_done::<crate::keys::QuotationEditModalKey>(
            &htmx,
            &QuotationDetailRouteTag::new(id).url(),
        ),
        Err(error) => {
            let fields = form_view_from_form(&state.db, &form).await;
            let page = QuotationEditModalPage {
                id,
                form_name: q.form_name(),
                number_label: form.number.clone(),
                fields,
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalFormQuery>,
) -> maud::Markup {
    let page = ConfirmDeletePage {
        id,
        form_name: q.form_name(),
        message: "Delete this quotation? Work orders that point at it keep their header."
            .to_string(),
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match delete_quotation(&state.db, id).await {
        Ok(()) => htmx.redirect(&QuotationDefaultRouteTag.url()),
        Err(error) => {
            let page = ConfirmDeletePage {
                id,
                form_name: String::new(),
                message: "Delete this quotation?".to_string(),
                error,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn copy_post(Cap(state): Cap<QuotationsState>, Path(id): Path<i64>) -> Response {
    match copy_quotation(&state.db, id).await {
        Ok(copied) => Redirect::to(&QuotationDetailRouteTag::new(copied.id).url()).into_response(),
        Err(_) => Redirect::to(&QuotationDefaultRouteTag.url()).into_response(),
    }
}

pub async fn create_invoice_get(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(quotation) = QuotationEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to(&QuotationDefaultRouteTag.url()).into_response();
    };
    let page = ConfirmInvoicePage {
        id,
        number: quotation.number,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_invoice_post(
    Cap(state): Cap<QuotationsState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(quotation) = QuotationEntity::find_by_id(id)
        .one(&state.db)
        .await
        .ok()
        .flatten()
    else {
        return Redirect::to(&QuotationDefaultRouteTag.url()).into_response();
    };
    let built = build_draft_from_quotation(&state.db, &quotation).await;
    match built {
        Ok(input) => match create_draft_invoice(&state.db, input, "UTC").await {
            Ok(draft) => {
                Redirect::to(&DraftInvoiceDetailRouteTag::new(draft.id).url()).into_response()
            }
            Err(error) => invoice_error_page(&chrome, &ctx, id, &quotation.number, error),
        },
        Err(error) => invoice_error_page(&chrome, &ctx, id, &quotation.number, error),
    }
}

fn invoice_error_page(
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    id: i64,
    number: &str,
    error: String,
) -> Response {
    let page = ConfirmInvoicePage {
        id,
        number: number.to_string(),
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}

async fn build_draft_from_quotation(
    db: &sea_orm::DatabaseConnection,
    quotation: &quotation::Model,
) -> Result<CreateDraftInput, String> {
    let payment_term_lines = parse_payment_term_lines_json(&default_payment_term_lines_json())?;
    let goods_rows = GoodsLineEntity::find()
        .filter(quotation_goods_line::Column::QuotationId.eq(quotation.id))
        .order_by_asc(quotation_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut lines = Vec::with_capacity(goods_rows.len());
    for line in goods_rows {
        let tax_ids = load_goods_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        lines.push(DraftLinePending {
            product_id: line.product_id,
            rate: Some(decimal_display(line.rate)),
            quantity: decimal_display(line.quantity),
            variables: serde_json::from_str(&line.variable_values).ok(),
            tax_ids: Some(tax_ids),
            remarks: line.remarks,
        });
    }
    let service_rows = ServiceLineEntity::find()
        .filter(quotation_service_line::Column::QuotationId.eq(quotation.id))
        .order_by_asc(quotation_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut service_lines = Vec::with_capacity(service_rows.len());
    for line in service_rows {
        let tax_ids = load_service_line_tax_ids(db, line.id)
            .await
            .unwrap_or_default();
        service_lines.push(DraftServiceLinePending {
            service_id: line.service_id,
            rate: Some(decimal_display(line.rate)),
            quantity: decimal_display(line.quantity),
            variables: serde_json::from_str(&line.variable_values).ok(),
            tax_ids: Some(tax_ids),
            remarks: line.remarks,
        });
    }
    let noon = NaiveTime::from_hms_opt(12, 0, 0).expect("12:00:00 is a valid time");
    Ok(CreateDraftInput {
        number: None,
        reference: Some(quotation.number.clone()),
        eway_bill_number: None,
        payment_reference: None,
        bank_account: quotation.bank_account.clone(),
        remarks: Some(quotation.remarks.clone()).filter(|text| !text.trim().is_empty()),
        datetime: quotation.date.and_time(noon).and_utc(),
        delivery_date: None,
        customer_id: quotation.customer_id,
        payment_term_lines,
        header_tax_ids: Vec::new(),
        lines,
        service_lines,
    })
}

async fn create_error_page(
    state: &QuotationsState,
    chrome: &SharedChromeFolder,
    ctx: &lariv_plugin_users::state::AuthContext,
    q: &ModalFormQuery,
    form: &QuotationForm,
    error: String,
) -> Response {
    let fields = form_view_from_form(&state.db, form).await;
    let page = QuotationCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        target_input: q.target_input(),
        fields,
        error,
    };
    html_built_page_with_slots(&page, chrome, &SlotCtx::from_auth(ctx)).into_response()
}
