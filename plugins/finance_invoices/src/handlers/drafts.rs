use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use maud::Markup;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder};

use lariv_core::components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::{CsrfToken, HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

use lariv_plugin_finance_accounts::journal_lines_editor::{PostJournalSection, post_journal_modal};
use lariv_plugin_finance_accounts::logic::custom_source_doc::{
    ensure_posting_accounts, parse_submitted_journal_lines,
};
use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_products::{entities::product::Entity as ProductEntity, pricing};
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::{
    draft_form_addon::{
        DraftInvoiceBulkEditFormPost, DraftInvoiceFormPost, render_draft_invoice_detail_extras,
        render_draft_invoice_form_extras, save_draft_invoice_form_extras,
        save_draft_invoice_form_extras_bulk,
    },
    entities::draft_invoice::{self, Entity as DraftInvoiceEntity},
    forms::{DraftInvoiceBulkEditForm, DraftInvoiceForm},
    keys::{
        DraftInvoiceBulkDeleteModalKey, DraftInvoiceBulkEditModalKey, DraftInvoiceBulkPostModalKey,
        DraftInvoiceCreateModalKey, DraftInvoiceDeleteModalKey, DraftInvoiceEditModalKey,
        DraftInvoicePostModalKey, DraftInvoiceSelectModalKey, DraftInvoiceSelectTableKey,
        InvoiceHubTableKey,
    },
    logic::bill_to::{self, PartyLabels},
    logic::draft_payment_term::draft_payment_term_display_rows,
    logic::invoice_goods_line_editor::{
        default_lines_json, draft_invoice_goods_line_display_rows, draft_lines_form_json,
        invoice_customer_name, invoice_goods_line_editor_preview_json,
    },
    logic::tax_assoc::load_draft_invoice_tax_ids,
    logic::{
        CreateDraftInput, PatchDraftInput, UpdateDraftInput, create_draft_invoice,
        default_payment_term_lines_json, delete_draft, format_delivery_date, format_invoice_date,
        load_invoice_date_formats, load_invoice_preferences, optional_display,
        optional_trimmed_text, parse_delivery_date, parse_invoice_datetime, parse_lines_json,
        parse_payment_term_lines_json, parse_service_lines_json, patch_draft_invoice,
        payment_term_lines_form_json, update_draft_invoice,
    },
    routes::{
        DraftInvoiceBulkPostRouteTag, DraftInvoiceDetailRouteTag, DraftInvoicePostRouteTag,
        PostedInvoiceDetailRouteTag,
    },
    scope::{find_active_draft, hub_tab_url},
    state::InvoicesState,
    templates::{
        ConfirmBulkDeletePage, ConfirmDeletePage, DraftInvoiceBulkEditModalPage,
        DraftInvoiceCreateModalPage, DraftInvoiceDetailPage, DraftInvoiceEditModalPage,
        DraftInvoiceSelectPage, DraftInvoiceSelectRow,
    },
};

use super::ModalNameQuery;

#[derive(Debug, serde::Deserialize)]
pub struct LinePriceQuery {
    product_id: i64,
    #[serde(default)]
    values: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ServiceLinePriceQuery {
    service_id: i64,
    #[serde(default)]
    values: String,
}

/// Evaluate a service price formula for the invoice line editor.
pub async fn service_line_price(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<ServiceLinePriceQuery>,
) -> Json<serde_json::Value> {
    use lariv_plugin_finance_services::entities::service::Entity as ServiceEntity;
    use lariv_plugin_finance_services::pricing as service_pricing;

    let raw = if q.values.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&q.values).unwrap_or_else(|_| serde_json::json!({}))
    };
    let service = ServiceEntity::find_by_id(q.service_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let Some(service) = service else {
        return Json(serde_json::json!({"pre_tax": "", "error": "unknown service"}));
    };
    if !service_pricing::has_variable_pricing(&service.variables) {
        return Json(serde_json::json!({"pre_tax": "", "error": ""}));
    }
    match service_pricing::price_line(&service.variables, &service.sales_price_formula, &raw) {
        Ok(priced) => Json(serde_json::json!({
            "pre_tax": decimal::decimal_display(priced.pre_tax),
            "error": "",
        })),
        Err(error) => Json(serde_json::json!({"pre_tax": "", "error": error})),
    }
}

/// Evaluate a product price formula for the invoice line editor.
pub async fn line_price(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<LinePriceQuery>,
) -> Json<serde_json::Value> {
    let raw = if q.values.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&q.values).unwrap_or_else(|_| serde_json::json!({}))
    };
    let product = ProductEntity::find_by_id(q.product_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();
    let Some(product) = product else {
        return Json(serde_json::json!({"pre_tax": "", "error": "unknown product"}));
    };
    if !pricing::has_variable_pricing(&product.variables) {
        return Json(serde_json::json!({"pre_tax": "", "error": ""}));
    }
    match pricing::price_line(&product.variables, &product.sales_price_formula, &raw) {
        Ok(priced) => Json(serde_json::json!({
            "pre_tax": decimal::decimal_display(priced.pre_tax),
            "error": "",
        })),
        Err(error) => Json(serde_json::json!({"pre_tax": "", "error": error})),
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkIdsQuery {
    #[serde(default)]
    pub ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkEditQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default)]
    pub ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkIdsForm {
    #[serde(default)]
    pub ids: String,
}

fn parse_bulk_ids(raw: &str) -> Vec<i64> {
    let mut ids: Vec<i64> = raw
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn bulk_ids_message(count: usize) -> String {
    if count == 1 {
        "Are you sure you want to delete the selected draft invoice?".into()
    } else {
        format!("Are you sure you want to delete {count} selected draft invoices?")
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct DeleteQuery {
    #[serde(default)]
    pub confirmed: Option<bool>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct DetailQuery {
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct DraftInvoiceSelectQuery {
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub page: QueryPage,
    #[serde(default)]
    pub page_size: QueryPageSize,
    #[serde(default)]
    pub target_input: Option<String>,
}

fn path_and_query(uri: &Uri) -> String {
    uri.path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn form_to_input(form: &DraftInvoiceForm, tz: &str) -> Result<CreateDraftInput, String> {
    let customer_id = form.customer_id;
    let payment_term_lines = parse_payment_term_lines_json(&form.payment_term_lines_json)?;
    let lines = parse_lines_json(&form.invoice_goods_lines_json)?;
    let service_lines = parse_service_lines_json(&form.invoice_service_lines_json)?;
    Ok(CreateDraftInput {
        number: Some(form.number.clone()),
        reference: optional_trimmed_text(&form.reference),
        eway_bill_number: optional_trimmed_text(&form.eway_bill_number),
        payment_reference: optional_trimmed_text(&form.payment_reference),
        bank_account: optional_trimmed_text(&form.bank_account),
        remarks: optional_trimmed_text(&form.remarks),
        datetime: parse_invoice_datetime(&form.datetime, tz),
        delivery_date: parse_delivery_date(&form.delivery_date)?,
        customer_id,
        payment_term_lines,
        header_tax_ids: form.taxes.clone(),
        lines,
        service_lines,
    })
}

fn blank_bulk_edit_form() -> DraftInvoiceBulkEditForm {
    DraftInvoiceBulkEditForm {
        number: String::new(),
        reference: String::new(),
        eway_bill_number: String::new(),
        payment_reference: String::new(),
        bank_account: String::new(),
        remarks: String::new(),
        datetime: String::new(),
        delivery_date: String::new(),
        customer_id: 0,
        payment_term_lines_json: default_payment_term_lines_json(),
        taxes: vec![],
        invoice_goods_lines_json: default_lines_json(),
        invoice_service_lines_json: crate::logic::invoice_service_line_editor::default_lines_json(),
        csrf: CsrfToken::default(),
    }
}

fn payment_term_lines_unchanged_from_blank(raw: &str) -> bool {
    let Ok(parsed) = parse_payment_term_lines_json(raw) else {
        return false;
    };
    let Ok(default) = parse_payment_term_lines_json(&default_payment_term_lines_json()) else {
        return false;
    };
    parsed == default
}

fn bulk_form_to_patch(
    form: &DraftInvoiceBulkEditForm,
    tz: &str,
) -> Result<PatchDraftInput, String> {
    let number = optional_trimmed_text(&form.number);
    let reference = optional_trimmed_text(&form.reference);
    let eway_bill_number = optional_trimmed_text(&form.eway_bill_number);
    let payment_reference = optional_trimmed_text(&form.payment_reference);
    let bank_account = optional_trimmed_text(&form.bank_account);
    let remarks = optional_trimmed_text(&form.remarks);
    let datetime = if form.datetime.trim().is_empty() {
        None
    } else {
        Some(parse_invoice_datetime(&form.datetime, tz))
    };
    let delivery_date = parse_delivery_date(&form.delivery_date)?;
    let customer_id = if form.customer_id > 0 {
        Some(form.customer_id)
    } else {
        None
    };
    let payment_term_lines = if form.payment_term_lines_json.trim().is_empty()
        || payment_term_lines_unchanged_from_blank(&form.payment_term_lines_json)
    {
        None
    } else {
        Some(parse_payment_term_lines_json(
            &form.payment_term_lines_json,
        )?)
    };
    let header_tax_ids = if form.taxes.is_empty() {
        None
    } else {
        Some(form.taxes.clone())
    };
    let lines = {
        let raw = form.invoice_goods_lines_json.trim();
        if raw.is_empty() {
            None
        } else {
            let parsed = parse_lines_json(raw)?;
            let filled: Vec<_> = parsed.into_iter().filter(|l| l.product_id > 0).collect();
            if filled.is_empty() {
                None
            } else {
                Some(filled)
            }
        }
    };
    let patch = PatchDraftInput {
        number,
        reference,
        eway_bill_number,
        payment_reference,
        bank_account,
        remarks,
        datetime,
        delivery_date,
        customer_id,
        payment_term_lines,
        header_tax_ids,
        lines,
        service_lines: {
            let raw = form.invoice_service_lines_json.trim();
            if raw.is_empty() {
                None
            } else {
                let parsed = parse_service_lines_json(raw)?;
                let filled: Vec<_> = parsed.into_iter().filter(|l| l.service_id > 0).collect();
                if filled.is_empty() {
                    None
                } else {
                    Some(filled)
                }
            }
        },
    };
    if patch.is_empty() {
        return Err("fill at least one field to update the selected drafts".to_string());
    }
    Ok(patch)
}

fn invoice_select_label(id: i64, number: &Option<String>) -> String {
    match number.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => n.to_string(),
        None => format!("#{id}"),
    }
}

struct DraftFormContext {
    customer_display: String,
    tax_items: Vec<ManyToManyItem>,
    invoice_goods_lines_preview: String,
    invoice_service_lines_preview: String,
    extra_inputs: String,
}

async fn load_draft_form_context(
    db: &sea_orm::DatabaseConnection,
    customer_id: i64,
    tax_ids: &[i64],
    draft_id: Option<i64>,
    posted: Option<&UrlencodedFields>,
) -> DraftFormContext {
    let customer_display = bill_to::customer_label(db, customer_id).await;

    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let tax_items = taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect();

    let invoice_goods_lines_preview = invoice_goods_line_editor_preview_json(db).await;
    let invoice_service_lines_preview =
        crate::logic::invoice_service_line_editor::invoice_service_line_editor_preview_json(db)
            .await;
    let extra_inputs = render_draft_invoice_form_extras(db, draft_id, posted).await;

    DraftFormContext {
        customer_display,
        tax_items,
        invoice_goods_lines_preview,
        invoice_service_lines_preview,
        extra_inputs,
    }
}

async fn draft_create_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    form: DraftInvoiceForm,
    error: String,
    posted: Option<&UrlencodedFields>,
) -> DraftInvoiceCreateModalPage {
    let ctx_data = load_draft_form_context(db, form.customer_id, &form.taxes, None, posted).await;
    DraftInvoiceCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form,
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_goods_lines_preview: ctx_data.invoice_goods_lines_preview,
        invoice_service_lines_preview: ctx_data.invoice_service_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
        error,
    }
}

async fn draft_edit_modal_page(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form_name: String,
    form: DraftInvoiceForm,
    error: String,
    posted: Option<&UrlencodedFields>,
) -> DraftInvoiceEditModalPage {
    let ctx_data =
        load_draft_form_context(db, form.customer_id, &form.taxes, Some(id), posted).await;
    DraftInvoiceEditModalPage {
        id,
        form_name,
        form,
        error,
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_goods_lines_preview: ctx_data.invoice_goods_lines_preview,
        invoice_service_lines_preview: ctx_data.invoice_service_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
    }
}

async fn draft_bulk_edit_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    ids: &str,
    selected_count: usize,
    form: DraftInvoiceBulkEditForm,
    error: String,
    can_submit: bool,
    posted: Option<&UrlencodedFields>,
) -> DraftInvoiceBulkEditModalPage {
    let ctx_data = load_draft_form_context(db, form.customer_id, &form.taxes, None, posted).await;
    DraftInvoiceBulkEditModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        ids: ids.to_string(),
        selected_count,
        form,
        customer_display: ctx_data.customer_display,
        tax_items: ctx_data.tax_items,
        invoice_goods_lines_preview: ctx_data.invoice_goods_lines_preview,
        invoice_service_lines_preview: ctx_data.invoice_service_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
        error,
        can_submit,
    }
}

pub async fn create_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let prefs = load_invoice_preferences(&state.db).await;
    let page = draft_create_modal_page(
        &state.db,
        &q,
        DraftInvoiceForm {
            number: String::new(),
            reference: String::new(),
            eway_bill_number: String::new(),
            payment_reference: String::new(),
            bank_account: prefs.default_bank_account.unwrap_or_default(),
            remarks: String::new(),
            datetime: format_invoice_date(Utc::now(), &ctx.timezone),
            delivery_date: String::new(),
            customer_id: 0,
            payment_term_lines_json: default_payment_term_lines_json(),
            taxes: vec![],
            invoice_goods_lines_json: default_lines_json(),
            invoice_service_lines_json:
                crate::logic::invoice_service_line_editor::default_lines_json(),
            csrf: CsrfToken::current(),
        },
        String::new(),
        None,
    )
    .await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    DraftInvoiceFormPost { form, fields }: DraftInvoiceFormPost,
) -> Response {
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => match create_draft_invoice(&state.db, input, &ctx.timezone).await {
            Ok(d) => {
                if let Err(e) = save_draft_invoice_form_extras(&state.db, d.id, &fields).await {
                    let page = draft_create_modal_page(&state.db, &q, form, e, Some(&fields)).await;
                    return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response();
                }
                respond_create_modal_done::<DraftInvoiceCreateModalKey>(
                    &htmx,
                    &q.refresh_table(),
                    &DraftInvoiceDetailRouteTag::new(d.id).url(),
                )
            }
            Err(e) => {
                let page =
                    draft_create_modal_page(&state.db, &q, form, e.to_string(), Some(&fields))
                        .await;
                html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response()
            }
        },
        Err(e) => {
            let page = draft_create_modal_page(&state.db, &q, form, e, Some(&fields)).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn detail(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(query): Query<DetailQuery>,
) -> Response {
    let Some(d) = find_active_draft(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    };

    let tax_ids = load_draft_invoice_tax_ids(&state.db, d.id)
        .await
        .unwrap_or_default();
    let taxes = load_taxes_by_ids(&state.db, &tax_ids)
        .await
        .unwrap_or_default();
    let tax_labels = if taxes.is_empty() {
        "—".to_string()
    } else {
        taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ")
    };

    let party = bill_to::load_bill_to(&state.db, d.customer_id).await;
    let customer_name = invoice_customer_name(&state.db, party).await;

    let dates = load_invoice_date_formats(&state.db).await;
    let payment_term_rows = draft_payment_term_display_rows(&state.db, d.id, &dates.date).await;
    let line_rows = draft_invoice_goods_line_display_rows(&state.db, d.id).await;
    let service_line_rows =
        crate::logic::invoice_service_line_editor::draft_invoice_service_line_display_rows(
            &state.db, d.id,
        )
        .await;
    let extra_detail = render_draft_invoice_detail_extras(&state.db, d.id).await;

    let page = DraftInvoiceDetailPage {
        id: d.id,
        number: d.number.unwrap_or_else(|| "—".to_string()),
        reference: optional_display(&d.reference),
        eway_bill_number: optional_display(&d.eway_bill_number),
        payment_reference: optional_display(&d.payment_reference),
        bank_account: optional_display(&d.bank_account),
        remarks: optional_display(&d.remarks),
        datetime: dates.datetime(d.datetime, &ctx.timezone),
        delivery_date: dates.calendar_or_dash(d.delivery_date),
        bill_to_individual: party.bill_to_individual,
        customer_id: party.party_id(),
        customer_name,
        payment_term_rows,
        tax_labels,
        extra_detail,
        line_rows,
        service_line_rows,
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinanceInvoicesMutate,
            >(),
        ),
        error: query.error.filter(|e| !e.is_empty()),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(d) = find_active_draft(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    };

    let tax_ids = load_draft_invoice_tax_ids(&state.db, d.id)
        .await
        .unwrap_or_default();
    let lines_json = draft_lines_form_json(&state.db, d.id).await;
    let service_lines_json =
        crate::logic::invoice_service_line_editor::draft_lines_form_json(&state.db, d.id).await;
    let payment_term_lines_json = payment_term_lines_form_json(&state.db, d.id).await;
    let form = DraftInvoiceForm {
        number: d.number.unwrap_or_default(),
        reference: d.reference.unwrap_or_default(),
        eway_bill_number: d.eway_bill_number.unwrap_or_default(),
        payment_reference: d.payment_reference.unwrap_or_default(),
        bank_account: d.bank_account.unwrap_or_default(),
        remarks: d.remarks.unwrap_or_default(),
        datetime: format_invoice_date(d.datetime, &ctx.timezone),
        delivery_date: format_delivery_date(d.delivery_date),
        customer_id: d.customer_id,
        payment_term_lines_json,
        taxes: tax_ids,
        invoice_goods_lines_json: lines_json,
        invoice_service_lines_json: service_lines_json,
        csrf: CsrfToken::current(),
    };

    let page = draft_edit_modal_page(&state.db, id, q.form_name(), form, String::new(), None).await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    DraftInvoiceFormPost { form, fields }: DraftInvoiceFormPost,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => {
            let update = UpdateDraftInput {
                number: input.number,
                reference: input.reference,
                eway_bill_number: input.eway_bill_number,
                payment_reference: input.payment_reference,
                bank_account: input.bank_account,
                remarks: input.remarks,
                datetime: input.datetime,
                delivery_date: input.delivery_date,
                customer_id: input.customer_id,
                payment_term_lines: input.payment_term_lines,
                header_tax_ids: input.header_tax_ids,
                lines: input.lines,
                service_lines: input.service_lines,
            };
            match update_draft_invoice(&state.db, id, update, &ctx.timezone).await {
                Ok(_) => {
                    if let Err(e) = save_draft_invoice_form_extras(&state.db, id, &fields).await {
                        let page = draft_edit_modal_page(
                            &state.db,
                            id,
                            q.form_name(),
                            form,
                            e,
                            Some(&fields),
                        )
                        .await;
                        return html_built_page_with_slots(
                            &page,
                            &chrome,
                            &SlotCtx::from_auth(&ctx),
                        )
                        .into_response();
                    }
                    respond_edit_modal_done::<DraftInvoiceEditModalKey>(
                        &htmx,
                        &DraftInvoiceDetailRouteTag::new(id).url(),
                    )
                }
                Err(e) => {
                    let page =
                        draft_edit_modal_page(&state.db, id, q.form_name(), form, e, Some(&fields))
                            .await;
                    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response()
                }
            }
        }
        Err(e) => {
            let page =
                draft_edit_modal_page(&state.db, id, q.form_name(), form, e, Some(&fields)).await;
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
        modal_uid: DraftInvoiceDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this draft invoice?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_invoices.DraftInvoiceDeleteForm".into()),
        id,
        error: String::new(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn delete_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match delete_draft(&state.db, id).await {
        Ok(_) => htmx.redirect(&hub_tab_url("drafts")),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete draft invoice");
            let page = ConfirmDeletePage {
                modal_uid: DraftInvoiceDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this draft invoice?".into(),
                form_name: "p_finance_invoices.DraftInvoiceDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

const POST_INTRO: &str = "These lines are filled from the product, service, and tax templates. Change them before posting. Saved lines are whatever you submit, and debits must equal credits.";

fn journal_field_name(id: i64, single: bool) -> String {
    if single {
        "JournalLines".to_string()
    } else {
        format!("JournalLines_{id}")
    }
}

async fn render_invoice_post_modal(
    db: &DatabaseConnection,
    ids: &[i64],
    single: bool,
    post_url: &str,
    modal_uid: &str,
    title: &str,
    submit_label: &str,
    overrides: &HashMap<i64, String>,
    form_error: &str,
) -> Markup {
    let mut sections = Vec::new();
    if ids.is_empty() {
        sections.push(PostJournalSection {
            title: "No drafts selected".into(),
            field_name: "JournalLines".into(),
            lines_json: String::new(),
            error: "Select at least one draft invoice to post.".into(),
        });
    }
    for id in ids {
        let field_name = journal_field_name(*id, single);
        match crate::logic::invoice_posting::preview_draft_journal(db, *id, &format!("draft-{id}"))
            .await
        {
            Ok(preview) => sections.push(PostJournalSection {
                title: preview.label,
                field_name,
                lines_json: overrides
                    .get(id)
                    .cloned()
                    .filter(|raw| !raw.trim().is_empty())
                    .unwrap_or(preview.lines_json),
                error: String::new(),
            }),
            Err(error) if error == "draft already posted" => {}
            Err(error) => sections.push(PostJournalSection {
                title: format!("Draft #{id}"),
                field_name,
                lines_json: String::new(),
                error,
            }),
        }
    }
    if sections.is_empty() {
        sections.push(PostJournalSection {
            title: "Nothing to post".into(),
            field_name: "JournalLines".into(),
            lines_json: String::new(),
            error: "These drafts are already posted.".into(),
        });
    }
    post_journal_modal(
        modal_uid,
        title,
        POST_INTRO,
        post_url,
        &sections,
        submit_label,
        form_error,
    )
}

pub async fn post_confirm_get(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    render_invoice_post_modal(
        &state.db,
        &[id],
        true,
        &DraftInvoicePostRouteTag::new(id).path(),
        DraftInvoicePostModalKey::ID,
        "Post invoice",
        "Post invoice",
        &HashMap::new(),
        "",
    )
    .await
}

pub async fn post_invoice(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    HtmlFormBody(fields): HtmlFormBody<HashMap<String, String>>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    let raw = fields.get("JournalLines").cloned().unwrap_or_default();
    let mut overrides = HashMap::new();
    if !raw.trim().is_empty() {
        overrides.insert(id, raw.clone());
    }
    let submitted = match parse_submitted_journal_lines(&raw) {
        Ok(lines) => lines,
        Err(error) => {
            return render_invoice_post_modal(
                &state.db,
                &[id],
                true,
                &DraftInvoicePostRouteTag::new(id).path(),
                DraftInvoicePostModalKey::ID,
                "Post invoice",
                "Post invoice",
                &overrides,
                &error,
            )
            .await
            .into_response();
        }
    };
    if let Err(error) = ensure_posting_accounts(&state.db, &submitted.lines).await {
        return render_invoice_post_modal(
            &state.db,
            &[id],
            true,
            &DraftInvoicePostRouteTag::new(id).path(),
            DraftInvoicePostModalKey::ID,
            "Post invoice",
            "Post invoice",
            &overrides,
            &error,
        )
        .await
        .into_response();
    }
    match crate::logic::invoice_posting::draft_new_posted_with_lines(
        &state.db,
        id,
        Utc::now(),
        &ctx.timezone,
        Some(submitted),
    )
    .await
    {
        Ok(posted) => htmx.redirect(&PostedInvoiceDetailRouteTag::new(posted.id).url()),
        Err(error) => render_invoice_post_modal(
            &state.db,
            &[id],
            true,
            &DraftInvoicePostRouteTag::new(id).path(),
            DraftInvoicePostModalKey::ID,
            "Post invoice",
            "Post invoice",
            &overrides,
            &error,
        )
        .await
        .into_response(),
    }
}

pub async fn bulk_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> maud::Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let page = ConfirmBulkDeletePage {
        modal_uid: DraftInvoiceBulkDeleteModalKey::ID.to_string(),
        message: if ids.is_empty() {
            "Select at least one draft invoice to delete.".into()
        } else {
            bulk_ids_message(ids.len())
        },
        form_name: "p_finance_invoices.DraftInvoiceBulkDeleteForm".into(),
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        error: if ids.is_empty() {
            "No invoices selected.".into()
        } else {
            String::new()
        },
        can_submit: !ids.is_empty(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn bulk_delete_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkIdsForm>,
) -> Response {
    let ids = parse_bulk_ids(&form.ids);
    if ids.is_empty() {
        let page = ConfirmBulkDeletePage {
            modal_uid: DraftInvoiceBulkDeleteModalKey::ID.to_string(),
            message: "Select at least one draft invoice to delete.".into(),
            form_name: "p_finance_invoices.DraftInvoiceBulkDeleteForm".into(),
            ids: String::new(),
            error: "No invoices selected.".into(),
            can_submit: false,
        };
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    for id in &ids {
        if find_active_draft(&state.db, *id).await.is_none() {
            continue;
        }
        if let Err(e) = delete_draft(&state.db, *id).await {
            tracing::error!(error = %e, id, "failed to bulk-delete draft invoice");
            let page = ConfirmBulkDeletePage {
                modal_uid: DraftInvoiceBulkDeleteModalKey::ID.to_string(),
                message: bulk_ids_message(ids.len()),
                form_name: "p_finance_invoices.DraftInvoiceBulkDeleteForm".into(),
                ids: ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                error: format!("Failed to delete draft #{id}: {e}"),
                can_submit: true,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }
    htmx.redirect(&hub_tab_url("drafts"))
}

pub async fn bulk_edit_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkEditQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let ids_str = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let (error, can_submit) = if ids.is_empty() {
        (
            "Select at least one draft invoice to edit.".to_string(),
            false,
        )
    } else {
        (String::new(), true)
    };
    let page = draft_bulk_edit_modal_page(
        &state.db,
        &q.modal,
        &ids_str,
        ids.len(),
        blank_bulk_edit_form(),
        error,
        can_submit,
        None,
    )
    .await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn bulk_edit_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(modal): Query<ModalNameQuery>,
    DraftInvoiceBulkEditFormPost {
        form,
        fields,
        ids: ids_raw,
    }: DraftInvoiceBulkEditFormPost,
) -> Response {
    let ids = parse_bulk_ids(&ids_raw);
    let ids_str = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    if ids.is_empty() {
        let page = draft_bulk_edit_modal_page(
            &state.db,
            &modal,
            &ids_str,
            0,
            form,
            "Select at least one draft invoice to edit.".into(),
            false,
            Some(&fields),
        )
        .await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }

    let patch = match bulk_form_to_patch(&form, &ctx.timezone) {
        Ok(p) => Some(p),
        Err(e) => {
            // Allow empty core fields when an addon still has values to apply.
            if e.contains("fill at least one field")
                && crate::draft_form_addon::addons_bulk_has_values(&fields)
            {
                None
            } else {
                let page = draft_bulk_edit_modal_page(
                    &state.db,
                    &modal,
                    &ids_str,
                    ids.len(),
                    form,
                    e,
                    true,
                    Some(&fields),
                )
                .await;
                return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response();
            }
        }
    };

    for id in &ids {
        if find_active_draft(&state.db, *id).await.is_none() {
            continue;
        }
        if let Some(ref patch) = patch {
            if let Err(e) = patch_draft_invoice(&state.db, *id, patch.clone(), &ctx.timezone).await
            {
                tracing::error!(error = %e, id, "failed to bulk-edit draft invoice");
                let page = draft_bulk_edit_modal_page(
                    &state.db,
                    &modal,
                    &ids_str,
                    ids.len(),
                    form,
                    format!("Failed to update draft #{id}: {e}"),
                    true,
                    Some(&fields),
                )
                .await;
                return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response();
            }
        }
        if let Err(e) = save_draft_invoice_form_extras_bulk(&state.db, *id, &fields).await {
            let page = draft_bulk_edit_modal_page(
                &state.db,
                &modal,
                &ids_str,
                ids.len(),
                form,
                format!("Failed to update draft #{id}: {e}"),
                true,
                Some(&fields),
            )
            .await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }

    let refresh = if modal.refresh_table().is_empty() {
        InvoiceHubTableKey::ID.to_string()
    } else {
        modal.refresh_table()
    };
    respond_create_modal_done::<DraftInvoiceBulkEditModalKey>(
        &htmx,
        &refresh,
        &hub_tab_url("drafts"),
    )
}

pub async fn bulk_post_confirm_get(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let post_url = if ids.is_empty() {
        DraftInvoiceBulkPostRouteTag.path()
    } else {
        bulk_transition_post_url(&DraftInvoiceBulkPostRouteTag.path(), &ids)
    };
    render_invoice_post_modal(
        &state.db,
        &ids,
        false,
        &post_url,
        DraftInvoiceBulkPostModalKey::ID,
        "Post selected invoices",
        "Post selected",
        &HashMap::new(),
        "",
    )
    .await
}

pub(crate) fn bulk_transition_post_url(path: &str, ids: &[i64]) -> String {
    let ids = ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{path}?ids={ids}")
}

pub async fn bulk_post(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkIdsQuery>,
    HtmlFormBody(fields): HtmlFormBody<HashMap<String, String>>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let post_url = bulk_transition_post_url(&DraftInvoiceBulkPostRouteTag.path(), &ids);
    if ids.is_empty() {
        return render_invoice_post_modal(
            &state.db,
            &ids,
            false,
            &post_url,
            DraftInvoiceBulkPostModalKey::ID,
            "Post selected invoices",
            "Post selected",
            &HashMap::new(),
            "Select at least one draft invoice to post.",
        )
        .await
        .into_response();
    }
    let mut overrides = HashMap::new();
    let mut parsed = Vec::new();
    for id in &ids {
        let raw = fields
            .get(&journal_field_name(*id, false))
            .cloned()
            .unwrap_or_default();
        if !raw.trim().is_empty() {
            overrides.insert(*id, raw.clone());
        }
        if find_active_draft(&state.db, *id).await.is_none() {
            continue;
        }
        let submitted = match parse_submitted_journal_lines(&raw) {
            Ok(lines) => lines,
            Err(error) => {
                return render_invoice_post_modal(
                    &state.db,
                    &ids,
                    false,
                    &post_url,
                    DraftInvoiceBulkPostModalKey::ID,
                    "Post selected invoices",
                    "Post selected",
                    &overrides,
                    &format!("Draft #{id}: {error}"),
                )
                .await
                .into_response();
            }
        };
        if let Err(error) = ensure_posting_accounts(&state.db, &submitted.lines).await {
            return render_invoice_post_modal(
                &state.db,
                &ids,
                false,
                &post_url,
                DraftInvoiceBulkPostModalKey::ID,
                "Post selected invoices",
                "Post selected",
                &overrides,
                &format!("Draft #{id}: {error}"),
            )
            .await
            .into_response();
        }
        parsed.push((*id, submitted));
    }
    let now = Utc::now();
    for (id, submitted) in parsed {
        if let Err(error) = crate::logic::invoice_posting::draft_new_posted_with_lines(
            &state.db,
            id,
            now,
            &ctx.timezone,
            Some(submitted),
        )
        .await
        {
            return render_invoice_post_modal(
                &state.db,
                &ids,
                false,
                &post_url,
                DraftInvoiceBulkPostModalKey::ID,
                "Post selected invoices",
                "Post selected",
                &overrides,
                &format!("Draft #{id}: {error}"),
            )
            .await
            .into_response();
        }
    }
    htmx.redirect(&hub_tab_url("posted"))
}

pub async fn multi_select(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<DraftInvoiceSelectQuery>,
) -> maud::Markup {
    let dates = load_invoice_date_formats(&state.db).await;
    let page_num = q.page.get();
    let mut query = DraftInvoiceEntity::find();
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => query.order_by_desc(draft_invoice::Column::Id),
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(draft_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(draft_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(draft_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(draft_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(draft_invoice::Column::Datetime)
        }
        _ => query.order_by_desc(draft_invoice::Column::Id),
    };
    let paginator = query.paginate(&state.db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let customer_ids: Vec<i64> = models.iter().map(|d| d.customer_id).collect();
    let parties = bill_to::parties_in_order(&state.db, &customer_ids).await;
    let customers = PartyLabels::load(&state.db, &parties).await;
    let rows: Vec<DraftInvoiceSelectRow> = models
        .into_iter()
        .zip(parties)
        .map(|(d, party)| DraftInvoiceSelectRow {
            id: d.id,
            number: invoice_select_label(d.id, &d.number),
            datetime: dates.datetime(d.datetime, &ctx.timezone),
            customer_name: customers.name(party),
        })
        .collect();
    let invoices = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = DraftInvoiceSelectPage {
        invoices,
        target_input: q.target_input.unwrap_or_else(|| "Invoices".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<DraftInvoiceSelectTableKey, DraftInvoiceSelectModalKey, _>(&htmx, &page)
}
