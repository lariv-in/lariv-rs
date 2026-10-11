use axum::{
    Json,
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};

use lariv_core::components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx, SwapKey};
use lariv_core::html_form::{CsrfToken, HtmlFormBody, UrlencodedFields};
use lariv_core::http::Cap;
use lariv_core::picker::respond_picker_select;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_users::middleware::RequireAuth;

use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_products::{entities::product::Entity as ProductEntity, pricing};
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};

use crate::{
    draft_form_addon::{
        DraftPurchaseBulkEditFormPost, DraftPurchaseFormPost, render_draft_purchase_detail_extras,
        render_draft_purchase_form_extras, save_draft_purchase_form_extras,
        save_draft_purchase_form_extras_bulk,
    },
    entities::draft_purchase::{self, Entity as DraftPurchaseEntity},
    forms::{DraftPurchaseBulkEditForm, DraftPurchaseForm},
    keys::{
        DraftPurchaseBulkDeleteModalKey, DraftPurchaseBulkEditModalKey, DraftPurchaseCreateModalKey,
        DraftPurchaseDeleteModalKey, DraftPurchaseEditModalKey, DraftPurchaseSelectModalKey,
        DraftPurchaseSelectTableKey, PurchaseHubTableKey,
    },
    logic::bill_to::{self, BillTo, PartyLabels},
    logic::draft_payment_term::draft_payment_term_display_rows,
    logic::purchase_line_editor::{
        default_lines_json, draft_purchase_line_display_rows, draft_lines_form_json,
        purchase_vendor_name, purchase_line_editor_preview_json,
    },
    logic::tax_assoc::load_draft_purchase_tax_ids,
    logic::{
        CreateDraftInput, PatchDraftInput, UpdateDraftInput, create_draft_purchase,
        default_payment_term_lines_json, delete_draft, format_delivery_date, format_purchase_date,
        load_purchase_date_formats, load_purchase_preferences, optional_display,
        optional_trimmed_text, parse_delivery_date, parse_purchase_datetime, parse_lines_json,
        parse_payment_term_lines_json, patch_draft_purchase, payment_term_lines_form_json,
        update_draft_purchase,
    },
    routes::{DraftPurchaseDetailRouteTag, PostedPurchaseDetailRouteTag},
    scope::{find_active_draft, hub_tab_url},
    state::PurchasesState,
    templates::{
        ConfirmBulkDeletePage, ConfirmDeletePage, DraftPurchaseBulkEditModalPage,
        DraftPurchaseCreateModalPage, DraftPurchaseDetailPage, DraftPurchaseEditModalPage,
        DraftPurchaseSelectPage, DraftPurchaseSelectRow,
    },
};

use super::ModalNameQuery;

#[derive(Debug, serde::Deserialize)]
pub struct LinePriceQuery {
    product_id: i64,
    #[serde(default)]
    values: String,
}

/// Evaluate a product price formula for the purchase line editor.
pub async fn line_price(
    Cap(state): Cap<PurchasesState>,
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
        "Are you sure you want to delete the selected draft purchase?".into()
    } else {
        format!("Are you sure you want to delete {count} selected draft purchases?")
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
pub struct DraftPurchaseSelectQuery {
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

fn form_to_input(form: &DraftPurchaseForm, tz: &str) -> Result<CreateDraftInput, String> {
    let bill_to = bill_to::require_bill_to(
        bill_to::checkbox_on(&form.vendor_is_individual),
        form.vendor_contact_id,
        form.vendor_company_id,
    )?;
    let payment_term_lines = parse_payment_term_lines_json(&form.payment_term_lines_json)?;
    let lines = parse_lines_json(&form.purchase_lines_json)?;
    Ok(CreateDraftInput {
        number: Some(form.number.clone()),
        reference: optional_trimmed_text(&form.reference),
        payment_reference: optional_trimmed_text(&form.payment_reference),
        bank_account: optional_trimmed_text(&form.bank_account),
        remarks: optional_trimmed_text(&form.remarks),
        datetime: parse_purchase_datetime(&form.datetime, tz),
        delivery_date: parse_delivery_date(&form.delivery_date)?,
        bill_to,
        payment_term_lines,
        header_tax_ids: form.taxes.clone(),
        lines,
    })
}

fn blank_bulk_edit_form() -> DraftPurchaseBulkEditForm {
    DraftPurchaseBulkEditForm {
        number: String::new(),
        reference: String::new(),
        payment_reference: String::new(),
        bank_account: String::new(),
        remarks: String::new(),
        datetime: String::new(),
        delivery_date: String::new(),
        vendor_is_individual: String::new(),
        vendor_contact_id: 0,
        vendor_company_id: 0,
        payment_term_lines_json: default_payment_term_lines_json(),
        taxes: vec![],
        purchase_lines_json: default_lines_json(),
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
    form: &DraftPurchaseBulkEditForm,
    tz: &str,
) -> Result<PatchDraftInput, String> {
    let number = optional_trimmed_text(&form.number);
    let reference = optional_trimmed_text(&form.reference);
    let payment_reference = optional_trimmed_text(&form.payment_reference);
    let bank_account = optional_trimmed_text(&form.bank_account);
    let remarks = optional_trimmed_text(&form.remarks);
    let datetime = if form.datetime.trim().is_empty() {
        None
    } else {
        Some(parse_purchase_datetime(&form.datetime, tz))
    };
    let delivery_date = parse_delivery_date(&form.delivery_date)?;
    let bill_to = bill_to::optional_bill_to(
        bill_to::checkbox_on(&form.vendor_is_individual),
        form.vendor_contact_id,
        form.vendor_company_id,
    )?;
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
        let raw = form.purchase_lines_json.trim();
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
        payment_reference,
        bank_account,
        remarks,
        datetime,
        delivery_date,
        bill_to,
        payment_term_lines,
        header_tax_ids,
        lines,
    };
    if patch.is_empty() {
        return Err("fill at least one field to update the selected drafts".to_string());
    }
    Ok(patch)
}

fn purchase_select_label(id: i64, number: &Option<String>) -> String {
    match number.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(n) => n.to_string(),
        None => format!("#{id}"),
    }
}

struct DraftFormContext {
    individual_display: String,
    company_display: String,
    tax_items: Vec<ManyToManyItem>,
    purchase_lines_preview: String,
    extra_inputs: String,
}

async fn load_draft_form_context(
    db: &sea_orm::DatabaseConnection,
    vendor_contact_id: i64,
    vendor_company_id: i64,
    tax_ids: &[i64],
    draft_id: Option<i64>,
    posted: Option<&UrlencodedFields>,
) -> DraftFormContext {
    let (individual_display, company_display) =
        bill_to::party_displays(db, vendor_contact_id, vendor_company_id).await;

    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let tax_items = taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect();

    let purchase_lines_preview = purchase_line_editor_preview_json(db).await;
    let extra_inputs = render_draft_purchase_form_extras(db, draft_id, posted).await;

    DraftFormContext {
        individual_display,
        company_display,
        tax_items,
        purchase_lines_preview,
        extra_inputs,
    }
}

async fn draft_create_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    form: DraftPurchaseForm,
    error: String,
    posted: Option<&UrlencodedFields>,
) -> DraftPurchaseCreateModalPage {
    let ctx_data = load_draft_form_context(
        db,
        form.vendor_contact_id,
        form.vendor_company_id,
        &form.taxes,
        None,
        posted,
    )
    .await;
    DraftPurchaseCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form,
        individual_display: ctx_data.individual_display,
        company_display: ctx_data.company_display,
        tax_items: ctx_data.tax_items,
        purchase_lines_preview: ctx_data.purchase_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
        error,
    }
}

async fn draft_edit_modal_page(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form_name: String,
    form: DraftPurchaseForm,
    error: String,
    posted: Option<&UrlencodedFields>,
) -> DraftPurchaseEditModalPage {
    let ctx_data = load_draft_form_context(
        db,
        form.vendor_contact_id,
        form.vendor_company_id,
        &form.taxes,
        Some(id),
        posted,
    )
    .await;
    DraftPurchaseEditModalPage {
        id,
        form_name,
        form,
        error,
        individual_display: ctx_data.individual_display,
        company_display: ctx_data.company_display,
        tax_items: ctx_data.tax_items,
        purchase_lines_preview: ctx_data.purchase_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
    }
}

async fn draft_bulk_edit_modal_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    ids: &str,
    selected_count: usize,
    form: DraftPurchaseBulkEditForm,
    error: String,
    can_submit: bool,
    posted: Option<&UrlencodedFields>,
) -> DraftPurchaseBulkEditModalPage {
    let ctx_data = load_draft_form_context(
        db,
        form.vendor_contact_id,
        form.vendor_company_id,
        &form.taxes,
        None,
        posted,
    )
    .await;
    DraftPurchaseBulkEditModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        ids: ids.to_string(),
        selected_count,
        form,
        individual_display: ctx_data.individual_display,
        company_display: ctx_data.company_display,
        tax_items: ctx_data.tax_items,
        purchase_lines_preview: ctx_data.purchase_lines_preview,
        extra_inputs: ctx_data.extra_inputs,
        error,
        can_submit,
    }
}

pub async fn create_get(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let prefs = load_purchase_preferences(&state.db).await;
    let page = draft_create_modal_page(
        &state.db,
        &q,
        DraftPurchaseForm {
            number: String::new(),
            reference: String::new(),
            payment_reference: String::new(),
            bank_account: prefs.default_bank_account.unwrap_or_default(),
            remarks: String::new(),
            datetime: format_purchase_date(Utc::now(), &ctx.timezone),
            delivery_date: String::new(),
            vendor_is_individual: String::new(),
            vendor_contact_id: 0,
            vendor_company_id: 0,
            payment_term_lines_json: default_payment_term_lines_json(),
            taxes: vec![],
            purchase_lines_json: default_lines_json(),
            csrf: CsrfToken::current(),
        },
        String::new(),
        None,
    )
    .await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<ModalNameQuery>,
    DraftPurchaseFormPost { form, fields }: DraftPurchaseFormPost,
) -> Response {
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => match create_draft_purchase(&state.db, input, &ctx.timezone).await {
            Ok(d) => {
                if let Err(e) = save_draft_purchase_form_extras(&state.db, d.id, &fields).await {
                    let page = draft_create_modal_page(&state.db, &q, form, e, Some(&fields)).await;
                    return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response();
                }
                respond_create_modal_done::<DraftPurchaseCreateModalKey>(
                    &htmx,
                    &q.refresh_table(),
                    &DraftPurchaseDetailRouteTag::new(d.id).url(),
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
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(query): Query<DetailQuery>,
) -> Response {
    let Some(d) = find_active_draft(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    };

    let tax_ids = load_draft_purchase_tax_ids(&state.db, d.id)
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

    let party = BillTo::new(
        d.vendor_is_individual,
        d.vendor_contact_id,
        d.vendor_company_id,
    );
    let vendor_name = purchase_vendor_name(&state.db, party).await;

    let dates = load_purchase_date_formats(&state.db).await;
    let payment_term_rows = draft_payment_term_display_rows(&state.db, d.id, &dates.date).await;
    let line_rows = draft_purchase_line_display_rows(&state.db, d.id).await;
    let extra_detail = render_draft_purchase_detail_extras(&state.db, d.id).await;

    let page = DraftPurchaseDetailPage {
        id: d.id,
        number: d.number.unwrap_or_else(|| "—".to_string()),
        reference: optional_display(&d.reference),
        payment_reference: optional_display(&d.payment_reference),
        bank_account: optional_display(&d.bank_account),
        remarks: optional_display(&d.remarks),
        datetime: dates.datetime(d.datetime, &ctx.timezone),
        delivery_date: dates.calendar_or_dash(d.delivery_date),
        vendor_is_individual: d.vendor_is_individual,
        vendor_id: party.party_id(),
        vendor_name,
        payment_term_rows,
        tax_labels,
        extra_detail,
        line_rows,
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinancePurchasesMutate,
            >(),
        ),
        error: query.error.filter(|e| !e.is_empty()),
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_get(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
) -> Response {
    let Some(d) = find_active_draft(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    };

    let tax_ids = load_draft_purchase_tax_ids(&state.db, d.id)
        .await
        .unwrap_or_default();
    let lines_json = draft_lines_form_json(&state.db, d.id).await;
    let payment_term_lines_json = payment_term_lines_form_json(&state.db, d.id).await;
    let party = bill_to::fill_company_from_contact(
        &state.db,
        BillTo::new(
            d.vendor_is_individual,
            d.vendor_contact_id,
            d.vendor_company_id,
        ),
    )
    .await;
    let form = DraftPurchaseForm {
        number: d.number.unwrap_or_default(),
        reference: d.reference.unwrap_or_default(),
        payment_reference: d.payment_reference.unwrap_or_default(),
        bank_account: d.bank_account.unwrap_or_default(),
        remarks: d.remarks.unwrap_or_default(),
        datetime: format_purchase_date(d.datetime, &ctx.timezone),
        delivery_date: format_delivery_date(d.delivery_date),
        vendor_is_individual: party.checkbox_value(),
        vendor_contact_id: party.vendor_contact_id.unwrap_or(0),
        vendor_company_id: party.vendor_company_id.unwrap_or(0),
        payment_term_lines_json,
        taxes: tax_ids,
        purchase_lines_json: lines_json,
        csrf: CsrfToken::current(),
    };

    let page = draft_edit_modal_page(&state.db, id, q.form_name(), form, String::new(), None).await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    DraftPurchaseFormPost { form, fields }: DraftPurchaseFormPost,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => {
            let update = UpdateDraftInput {
                number: input.number,
                reference: input.reference,
                payment_reference: input.payment_reference,
                bank_account: input.bank_account,
                remarks: input.remarks,
                datetime: input.datetime,
                delivery_date: input.delivery_date,
                bill_to: input.bill_to,
                payment_term_lines: input.payment_term_lines,
                header_tax_ids: input.header_tax_ids,
                lines: input.lines,
            };
            match update_draft_purchase(&state.db, id, update, &ctx.timezone).await {
                Ok(_) => {
                    if let Err(e) = save_draft_purchase_form_extras(&state.db, id, &fields).await {
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
                    respond_edit_modal_done::<DraftPurchaseEditModalKey>(
                        &htmx,
                        &DraftPurchaseDetailRouteTag::new(id).url(),
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
        modal_uid: DraftPurchaseDeleteModalKey::ID.to_string(),
        message: "Are you sure you want to delete this draft purchase?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_purchases.DraftPurchaseDeleteForm".into()),
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
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match delete_draft(&state.db, id).await {
        Ok(_) => htmx.redirect(&hub_tab_url("drafts")),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to delete draft purchase");
            let page = ConfirmDeletePage {
                modal_uid: DraftPurchaseDeleteModalKey::ID.to_string(),
                message: "Are you sure you want to delete this draft purchase?".into(),
                form_name: "p_finance_purchases.DraftPurchaseDeleteForm".into(),
                id,
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn post_purchase(
    Cap(state): Cap<PurchasesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if find_active_draft(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    match crate::logic::draft_new_posted(&state.db, id, Utc::now(), &ctx.timezone).await {
        Ok(p) => Redirect::to(&PostedPurchaseDetailRouteTag::new(p.id).url()).into_response(),
        Err(e) => Redirect::to(
            &DraftPurchaseDetailRouteTag::new(id)
                .with_query()
                .query("error", &e)
                .build(),
        )
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
        modal_uid: DraftPurchaseBulkDeleteModalKey::ID.to_string(),
        message: if ids.is_empty() {
            "Select at least one draft purchase to delete.".into()
        } else {
            bulk_ids_message(ids.len())
        },
        form_name: "p_finance_purchases.DraftPurchaseBulkDeleteForm".into(),
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        error: if ids.is_empty() {
            "No purchases selected.".into()
        } else {
            String::new()
        },
        can_submit: !ids.is_empty(),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
}

pub async fn bulk_delete_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkIdsForm>,
) -> Response {
    let ids = parse_bulk_ids(&form.ids);
    if ids.is_empty() {
        let page = ConfirmBulkDeletePage {
            modal_uid: DraftPurchaseBulkDeleteModalKey::ID.to_string(),
            message: "Select at least one draft purchase to delete.".into(),
            form_name: "p_finance_purchases.DraftPurchaseBulkDeleteForm".into(),
            ids: String::new(),
            error: "No purchases selected.".into(),
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
            tracing::error!(error = %e, id, "failed to bulk-delete draft purchase");
            let page = ConfirmBulkDeletePage {
                modal_uid: DraftPurchaseBulkDeleteModalKey::ID.to_string(),
                message: bulk_ids_message(ids.len()),
                form_name: "p_finance_purchases.DraftPurchaseBulkDeleteForm".into(),
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
    Cap(state): Cap<PurchasesState>,
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
            "Select at least one draft purchase to edit.".to_string(),
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
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(modal): Query<ModalNameQuery>,
    DraftPurchaseBulkEditFormPost {
        form,
        fields,
        ids: ids_raw,
    }: DraftPurchaseBulkEditFormPost,
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
            "Select at least one draft purchase to edit.".into(),
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
            if let Err(e) = patch_draft_purchase(&state.db, *id, patch.clone(), &ctx.timezone).await
            {
                tracing::error!(error = %e, id, "failed to bulk-edit draft purchase");
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
        if let Err(e) = save_draft_purchase_form_extras_bulk(&state.db, *id, &fields).await {
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
        PurchaseHubTableKey::ID.to_string()
    } else {
        modal.refresh_table()
    };
    respond_create_modal_done::<DraftPurchaseBulkEditModalKey>(
        &htmx,
        &refresh,
        &hub_tab_url("drafts"),
    )
}

pub async fn bulk_post(
    Cap(state): Cap<PurchasesState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkIdsQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return Redirect::to(&hub_tab_url("drafts")).into_response();
    }
    let now = Utc::now();
    for id in ids {
        if find_active_draft(&state.db, id).await.is_none() {
            continue;
        }
        if let Err(e) = crate::logic::draft_new_posted(&state.db, id, now, &ctx.timezone).await {
            return Redirect::to(
                &DraftPurchaseDetailRouteTag::new(id)
                    .with_query()
                    .query("error", &e)
                    .build(),
            )
            .into_response();
        }
    }
    Redirect::to(&hub_tab_url("posted")).into_response()
}

pub async fn multi_select(
    Cap(state): Cap<PurchasesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<DraftPurchaseSelectQuery>,
) -> maud::Markup {
    let dates = load_purchase_date_formats(&state.db).await;
    let page_num = q.page.get();
    let mut query = DraftPurchaseEntity::find();
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => query.order_by_desc(draft_purchase::Column::Id),
        s if s.eq_ignore_ascii_case("ID ASC") || s.eq_ignore_ascii_case("ID") => {
            query.order_by_asc(draft_purchase::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(draft_purchase::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(draft_purchase::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(draft_purchase::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(draft_purchase::Column::Datetime)
        }
        _ => query.order_by_desc(draft_purchase::Column::Id),
    };
    let paginator = query.paginate(&state.db, q.page_size.get() as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let parties: Vec<BillTo> = models
        .iter()
        .map(|d| {
            BillTo::new(
                d.vendor_is_individual,
                d.vendor_contact_id,
                d.vendor_company_id,
            )
        })
        .collect();
    let vendors = PartyLabels::load(&state.db, &parties).await;
    let rows: Vec<DraftPurchaseSelectRow> = models
        .into_iter()
        .zip(parties)
        .map(|(d, party)| DraftPurchaseSelectRow {
            id: d.id,
            number: purchase_select_label(d.id, &d.number),
            datetime: dates.datetime(d.datetime, &ctx.timezone),
            vendor_name: vendors.name(party),
        })
        .collect();
    let purchases = ObjectList::from_page(rows, page_num, q.page_size.get(), total);
    let page = DraftPurchaseSelectPage {
        purchases,
        target_input: q.target_input.unwrap_or_else(|| "Purchases".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size: q.page_size.get(),
    };
    respond_picker_select::<DraftPurchaseSelectTableKey, DraftPurchaseSelectModalKey, _>(&htmx, &page)
}
