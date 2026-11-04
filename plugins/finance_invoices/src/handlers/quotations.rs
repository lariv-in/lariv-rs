use axum::{
    extract::{Path, Query},
    http::Uri,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use maud::Markup;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use lariv_core::components::{ManyToManyItem, ObjectList, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::CsrfToken;
use lariv_core::http::Cap;
use lariv_core::list_filters::apply_filters;
use lariv_core::picker::respond_picker_select;
use lariv_core::web::{
    Htmx, QueryPage, QueryPageSize, html_built_page_or_app_layout, html_built_page_with_slots,
    respond_create_modal_done, respond_edit_modal_done,
};
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    draft_form_addon::{DraftInvoiceBulkEditFormPost, DraftInvoiceFormPost},
    entities::quotation_invoice::{self, Entity as QuotationInvoiceEntity},
    forms::DraftInvoiceForm,
    hub_filter::{HubFilterKind, hub_filter_fields, invoice_picker_clauses},
    hub_filter_addon::{HubQueryParams, apply_extra_sql_filters, render_extra_filter_inputs},
    keys::{
        QUOTATION_INVOICE_BULK_CREATE_DRAFT_MODAL, QUOTATION_INVOICE_BULK_DELETE_MODAL,
        QUOTATION_INVOICE_BULK_EDIT_MODAL, QUOTATION_INVOICE_CREATE_DRAFT_MODAL,
        QUOTATION_INVOICE_CREATE_MODAL, QUOTATION_INVOICE_DELETE_MODAL,
        QUOTATION_INVOICE_EDIT_MODAL, QUOTATION_INVOICE_SELECT_MODAL,
        QUOTATION_INVOICE_SELECT_TABLE,
    },
    logic::bill_to::{self, PartyLabels},
    logic::invoice_line_editor::{
        default_lines_json, invoice_customer_name, invoice_line_editor_preview_json,
        quotation_invoice_line_display_rows, quotation_lines_form_json,
    },
    logic::quotation::{
        create_quotation_invoice, delete_quotation, patch_quotation_invoice,
        quotation_payment_term_display_rows, quotation_payment_term_lines_form_json,
        quotation_to_draft, update_quotation_invoice,
    },
    logic::tax_assoc::load_quotation_invoice_tax_ids,
    logic::{
        UpdateDraftInput, default_payment_term_lines_json, format_delivery_date,
        format_invoice_date, load_invoice_date_formats, load_invoice_preferences, optional_display,
    },
    routes::{
        DraftInvoiceDetailRouteTag, QuotationInvoiceBulkCreateDraftRouteTag,
        QuotationInvoiceBulkDeletePostRouteTag, QuotationInvoiceCreateDraftRouteTag,
        QuotationInvoiceDeletePostRouteTag, QuotationInvoiceDetailRouteTag,
    },
    scope::{find_active_quotation, hub_tab_url},
    state::InvoicesState,
    templates::{
        ConfirmBulkDeletePage, ConfirmDeletePage, ConfirmInvoiceTransition,
        QuotationInvoiceBulkEditModalPage, QuotationInvoiceCreateModalPage,
        QuotationInvoiceDetailPage, QuotationInvoiceEditModalPage, QuotationInvoiceSelectPage,
        QuotationInvoiceSelectRow, confirm_invoice_transition,
    },
};

use super::ModalNameQuery;
use super::drafts::{
    BulkIdsForm, blank_bulk_edit_form, bulk_form_to_patch, bulk_transition_post_url, form_to_input,
    invoice_select_label, parse_bulk_ids, path_and_query,
};
use lariv_core::html_form::HtmlFormBody;

fn query_escape(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct DetailQuery {
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct CreateQuery {
    #[serde(flatten)]
    modal: ModalNameQuery,
    #[serde(default, rename = "CustomerID")]
    customer_id: Option<i64>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct IdsQuery {
    #[serde(default)]
    ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkEditQuery {
    #[serde(flatten)]
    modal: ModalNameQuery,
    #[serde(default)]
    ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct SelectQuery {
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    page: QueryPage,
    #[serde(default)]
    page_size: QueryPageSize,
    #[serde(default)]
    target_input: Option<String>,
}

struct FormContext {
    customer_display: String,
    tax_items: Vec<ManyToManyItem>,
    invoice_lines_preview: String,
}

async fn form_context(
    db: &sea_orm::DatabaseConnection,
    customer_id: i64,
    tax_ids: &[i64],
) -> FormContext {
    let customer_display = bill_to::customer_label(db, customer_id).await;
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    let tax_items = taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect();
    let invoice_lines_preview = invoice_line_editor_preview_json(db).await;
    FormContext {
        customer_display,
        tax_items,
        invoice_lines_preview,
    }
}

async fn create_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    form: DraftInvoiceForm,
    error: String,
) -> QuotationInvoiceCreateModalPage {
    let ctx = form_context(db, form.customer_id, &form.taxes).await;
    QuotationInvoiceCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form,
        customer_display: ctx.customer_display,
        tax_items: ctx.tax_items,
        invoice_lines_preview: ctx.invoice_lines_preview,
        extra_inputs: String::new(),
        error,
    }
}

async fn edit_page(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    form_name: String,
    form: DraftInvoiceForm,
    error: String,
) -> QuotationInvoiceEditModalPage {
    let ctx = form_context(db, form.customer_id, &form.taxes).await;
    QuotationInvoiceEditModalPage {
        id,
        form_name,
        form,
        error,
        customer_display: ctx.customer_display,
        tax_items: ctx.tax_items,
        invoice_lines_preview: ctx.invoice_lines_preview,
        extra_inputs: String::new(),
    }
}

fn can_edit() -> bool {
    lariv_core::components::role_permitted(&lariv_plugin_users::role_authorization::roles_for::<
        crate::routes::FinanceInvoicesMutate,
    >())
}

pub async fn create_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<CreateQuery>,
) -> Response {
    let prefs = load_invoice_preferences(&state.db).await;
    let customer_id = q.customer_id.filter(|id| *id > 0).unwrap_or(0);
    let page = create_page(
        &state.db,
        &q.modal,
        DraftInvoiceForm {
            number: String::new(),
            reference: String::new(),
            eway_bill_number: String::new(),
            payment_reference: String::new(),
            bank_account: prefs.default_bank_account.unwrap_or_default(),
            remarks: String::new(),
            datetime: format_invoice_date(Utc::now(), &ctx.timezone),
            delivery_date: String::new(),
            customer_id,
            payment_term_lines_json: default_payment_term_lines_json(),
            taxes: vec![],
            invoice_lines_json: default_lines_json(),
            csrf: CsrfToken::current(),
        },
        String::new(),
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
    DraftInvoiceFormPost { form, fields: _ }: DraftInvoiceFormPost,
) -> Response {
    match form_to_input(&form, &ctx.timezone) {
        Ok(input) => match create_quotation_invoice(&state.db, input, &ctx.timezone).await {
            Ok(row) => respond_create_modal_done(
                &htmx,
                QUOTATION_INVOICE_CREATE_MODAL,
                &q.refresh_table(),
                &QuotationInvoiceDetailRouteTag::new(row.id).url(),
            ),
            Err(e) => {
                let page = create_page(&state.db, &q, form, e).await;
                html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                    .into_response()
            }
        },
        Err(e) => {
            let page = create_page(&state.db, &q, form, e).await;
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
    let Some(row) = find_active_quotation(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("quotations")).into_response();
    };
    let tax_ids = load_quotation_invoice_tax_ids(&state.db, row.id)
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
    let party = bill_to::load_bill_to(&state.db, row.customer_id).await;
    let customer_name = invoice_customer_name(&state.db, party).await;
    let dates = load_invoice_date_formats(&state.db).await;
    let page = QuotationInvoiceDetailPage {
        id: row.id,
        number: row.number.unwrap_or_else(|| "—".to_string()),
        reference: optional_display(&row.reference),
        eway_bill_number: optional_display(&row.eway_bill_number),
        payment_reference: optional_display(&row.payment_reference),
        bank_account: optional_display(&row.bank_account),
        remarks: optional_display(&row.remarks),
        datetime: dates.datetime(row.datetime, &ctx.timezone),
        delivery_date: dates.calendar_or_dash(row.delivery_date),
        bill_to_individual: party.bill_to_individual,
        customer_id: party.party_id(),
        customer_name,
        payment_term_rows: quotation_payment_term_display_rows(&state.db, row.id, &dates.date)
            .await,
        tax_labels,
        extra_detail: String::new(),
        line_rows: quotation_invoice_line_display_rows(&state.db, row.id).await,
        can_edit: can_edit(),
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
    let Some(row) = find_active_quotation(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("quotations")).into_response();
    };
    let tax_ids = load_quotation_invoice_tax_ids(&state.db, row.id)
        .await
        .unwrap_or_default();
    let form = DraftInvoiceForm {
        number: row.number.unwrap_or_default(),
        reference: row.reference.unwrap_or_default(),
        eway_bill_number: row.eway_bill_number.unwrap_or_default(),
        payment_reference: row.payment_reference.unwrap_or_default(),
        bank_account: row.bank_account.unwrap_or_default(),
        remarks: row.remarks.unwrap_or_default(),
        datetime: format_invoice_date(row.datetime, &ctx.timezone),
        delivery_date: format_delivery_date(row.delivery_date),
        customer_id: row.customer_id,
        payment_term_lines_json: quotation_payment_term_lines_form_json(&state.db, row.id).await,
        taxes: tax_ids,
        invoice_lines_json: quotation_lines_form_json(&state.db, row.id).await,
        csrf: CsrfToken::current(),
    };
    let page = edit_page(&state.db, id, q.form_name(), form, String::new()).await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn edit_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<ModalNameQuery>,
    DraftInvoiceFormPost { form, fields: _ }: DraftInvoiceFormPost,
) -> Response {
    if find_active_quotation(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("quotations")).into_response();
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
            };
            match update_quotation_invoice(&state.db, id, update, &ctx.timezone).await {
                Ok(_) => respond_edit_modal_done(
                    &htmx,
                    QUOTATION_INVOICE_EDIT_MODAL,
                    &QuotationInvoiceDetailRouteTag::new(id).url(),
                ),
                Err(e) => {
                    let page = edit_page(&state.db, id, q.form_name(), form, e).await;
                    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                        .into_response()
                }
            }
        }
        Err(e) => {
            let page = edit_page(&state.db, id, q.form_name(), form, e).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<ModalNameQuery>,
    Path(id): Path<i64>,
) -> Markup {
    let page = ConfirmDeletePage {
        modal_uid: QUOTATION_INVOICE_DELETE_MODAL.id().to_string(),
        message: "Are you sure you want to delete this quotation?".into(),
        form_name: q
            .name
            .clone()
            .unwrap_or_else(|| "p_finance_invoices.QuotationInvoiceDeleteForm".into()),
        id,
        post_url: QuotationInvoiceDeletePostRouteTag::new(id).url(),
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
    if find_active_quotation(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("quotations")).into_response();
    }
    match delete_quotation(&state.db, id).await {
        Ok(()) => htmx.redirect(&hub_tab_url("quotations")),
        Err(e) => {
            let page = ConfirmDeletePage {
                modal_uid: QUOTATION_INVOICE_DELETE_MODAL.id().to_string(),
                message: "Are you sure you want to delete this quotation?".into(),
                form_name: "p_finance_invoices.QuotationInvoiceDeleteForm".into(),
                id,
                post_url: QuotationInvoiceDeletePostRouteTag::new(id).url(),
                error: e,
            };
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn create_draft_get(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Markup {
    let found = find_active_quotation(&state.db, id).await.is_some();
    confirm_invoice_transition(ConfirmInvoiceTransition {
        modal_uid: QUOTATION_INVOICE_CREATE_DRAFT_MODAL.id(),
        title: "Create draft invoice",
        message: if found {
            "Copy this quotation into a new draft invoice. The quotation stays as it is."
        } else {
            "Quotation not found."
        },
        submit_label: "Create draft",
        post_url: &QuotationInvoiceCreateDraftRouteTag::new(id).path(),
        can_submit: found,
    })
}

pub async fn create_draft_post(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    match quotation_to_draft(&state.db, id).await {
        Ok(draft) => htmx.redirect(&DraftInvoiceDetailRouteTag::new(draft.id).url()),
        Err(e) => htmx.redirect(&format!(
            "{}?error={}",
            QuotationInvoiceDetailRouteTag::new(id).url(),
            query_escape(&e)
        )),
    }
}

pub async fn bulk_delete_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<IdsQuery>,
) -> Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let page = ConfirmBulkDeletePage {
        modal_uid: QUOTATION_INVOICE_BULK_DELETE_MODAL.id().to_string(),
        message: if ids.is_empty() {
            "Select at least one quotation to delete.".into()
        } else if ids.len() == 1 {
            "Are you sure you want to delete the selected quotation?".into()
        } else {
            format!(
                "Are you sure you want to delete {} selected quotations?",
                ids.len()
            )
        },
        form_name: "p_finance_invoices.QuotationInvoiceBulkDeleteForm".into(),
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        post_url: QuotationInvoiceBulkDeletePostRouteTag.url(),
        error: if ids.is_empty() {
            "No quotations selected.".into()
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
        return Redirect::to(&hub_tab_url("quotations")).into_response();
    }
    for id in &ids {
        if let Err(e) = delete_quotation(&state.db, *id).await {
            let page = ConfirmBulkDeletePage {
                modal_uid: QUOTATION_INVOICE_BULK_DELETE_MODAL.id().to_string(),
                message: "Could not delete every selected quotation.".into(),
                form_name: "p_finance_invoices.QuotationInvoiceBulkDeleteForm".into(),
                ids: ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                post_url: QuotationInvoiceBulkDeletePostRouteTag.url(),
                error: format!("Failed to delete quotation #{id}: {e}"),
                can_submit: true,
            };
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }
    htmx.redirect(&hub_tab_url("quotations"))
}

async fn bulk_edit_page(
    db: &sea_orm::DatabaseConnection,
    q: &ModalNameQuery,
    ids: &str,
    selected_count: usize,
    error: String,
    can_submit: bool,
) -> QuotationInvoiceBulkEditModalPage {
    let form = blank_bulk_edit_form();
    let ctx = form_context(db, form.customer_id, &form.taxes).await;
    QuotationInvoiceBulkEditModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        ids: ids.to_string(),
        selected_count,
        form,
        customer_display: ctx.customer_display,
        tax_items: ctx.tax_items,
        invoice_lines_preview: ctx.invoice_lines_preview,
        extra_inputs: String::new(),
        error,
        can_submit,
    }
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
        ("Select at least one quotation to edit.".to_string(), false)
    } else {
        (String::new(), true)
    };
    let page = bulk_edit_page(&state.db, &q.modal, &ids_str, ids.len(), error, can_submit).await;
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
        fields: _,
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
        let page = bulk_edit_page(
            &state.db,
            &modal,
            &ids_str,
            0,
            "Select at least one quotation to edit.".into(),
            false,
        )
        .await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let patch = match bulk_form_to_patch(&form, &ctx.timezone) {
        Ok(patch) => patch,
        Err(e) => {
            let page = bulk_edit_page(&state.db, &modal, &ids_str, ids.len(), e, true).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    for id in &ids {
        if let Err(e) = patch_quotation_invoice(&state.db, *id, patch.clone(), &ctx.timezone).await
        {
            let page = bulk_edit_page(
                &state.db,
                &modal,
                &ids_str,
                ids.len(),
                format!("Failed to update quotation #{id}: {e}"),
                true,
            )
            .await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }
    respond_edit_modal_done(
        &htmx,
        QUOTATION_INVOICE_BULK_EDIT_MODAL,
        &hub_tab_url("quotations"),
    )
}

pub async fn bulk_create_draft_get(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<IdsQuery>,
) -> Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let mut found = 0;
    for id in &ids {
        if find_active_quotation(&state.db, *id).await.is_some() {
            found += 1;
        }
    }
    let post_url = bulk_transition_post_url(&QuotationInvoiceBulkCreateDraftRouteTag.path(), &ids);
    confirm_invoice_transition(ConfirmInvoiceTransition {
        modal_uid: QUOTATION_INVOICE_BULK_CREATE_DRAFT_MODAL.id(),
        title: "Create draft invoices",
        message: if found == 0 {
            "Select at least one quotation."
        } else if found == 1 {
            "Copy the selected quotation into a new draft invoice. The quotation stays as it is."
        } else {
            "Copy each selected quotation into a new draft invoice. The quotations stay as they are."
        },
        submit_label: "Create drafts",
        post_url: &post_url,
        can_submit: found > 0,
    })
}

pub async fn bulk_create_draft_post(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<IdsQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    for id in &ids {
        if let Err(e) = quotation_to_draft(&state.db, *id).await {
            return htmx.redirect(&format!(
                "{}?error={}",
                hub_tab_url("quotations"),
                query_escape(&e)
            ));
        }
    }
    htmx.redirect(&hub_tab_url("drafts"))
}

pub async fn multi_select(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    uri: Uri,
    Query(q): Query<SelectQuery>,
) -> Markup {
    let dates = load_invoice_date_formats(&state.db).await;
    let page_num = q.page.get();
    let page_size = q.page_size.get();
    let query_string = uri.query().unwrap_or("");
    let clauses = invoice_picker_clauses(query_string);
    let fields = hub_filter_fields(HubFilterKind::Quotation, &ctx.timezone);
    let applied = apply_filters(state.db.get_database_backend(), &fields, &clauses);
    let params = HubQueryParams::new(query_string);
    let mut query = QuotationInvoiceEntity::find().filter(applied.condition.clone());
    query = apply_extra_sql_filters(query, &params, "quotation_invoices.id");
    let sort = q.sort.as_deref().unwrap_or("").trim();
    query = match sort {
        s if s.eq_ignore_ascii_case("ID DESC") => {
            query.order_by_desc(quotation_invoice::Column::Id)
        }
        s if s.eq_ignore_ascii_case("Number DESC") => {
            query.order_by_desc(quotation_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Number ASC") || s.eq_ignore_ascii_case("Number") => {
            query.order_by_asc(quotation_invoice::Column::Number)
        }
        s if s.eq_ignore_ascii_case("Date DESC") => {
            query.order_by_desc(quotation_invoice::Column::Datetime)
        }
        s if s.eq_ignore_ascii_case("Date ASC") || s.eq_ignore_ascii_case("Date") => {
            query.order_by_asc(quotation_invoice::Column::Datetime)
        }
        _ => query.order_by_desc(quotation_invoice::Column::Id),
    };
    let paginator = query.paginate(&state.db, page_size as u64);
    let total = paginator.num_items().await.unwrap_or(0);
    let models = paginator
        .fetch_page((page_num as u64).saturating_sub(1))
        .await
        .unwrap_or_default();
    let customer_ids: Vec<i64> = models.iter().map(|d| d.customer_id).collect();
    let parties = bill_to::parties_in_order(&state.db, &customer_ids).await;
    let customers = PartyLabels::load(&state.db, &parties).await;
    let rows: Vec<QuotationInvoiceSelectRow> = models
        .into_iter()
        .zip(parties)
        .map(|(d, party)| QuotationInvoiceSelectRow {
            id: d.id,
            number: invoice_select_label(d.id, &d.number),
            datetime: dates.datetime(d.datetime, &ctx.timezone),
            customer_name: customers.name(party),
        })
        .collect();
    let invoices = ObjectList::from_page(rows, page_num, page_size, total);
    let filters = super::hub::filter_clause_views(&state.db, &clauses, &applied.errors).await;
    let extra_filters = render_extra_filter_inputs(&state.db, &params)
        .await
        .into_string();
    let page = QuotationInvoiceSelectPage {
        invoices,
        fields,
        filters,
        filter_errors: applied.errors,
        extra_filters,
        target_input: q.target_input.unwrap_or_else(|| "Invoices".into()),
        sort: q.sort.clone().unwrap_or_default(),
        path_and_query: path_and_query(&uri),
        page_size,
    };
    respond_picker_select(
        &htmx,
        QUOTATION_INVOICE_SELECT_TABLE,
        QUOTATION_INVOICE_SELECT_MODAL,
        &page,
    )
}
