use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots},
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::scope::load_journal_entry_currency_format;

use crate::plugins::finance_invoices::{
    forms::CancelInvoiceForm,
    logic::{
        draft_payment_term::posted_payment_term_display_rows,
        format_delivery_date, format_invoice_date,
        invoice_line_editor::{
            invoice_customer_name, invoice_header_tax_labels, posted_invoice_line_display_rows,
        },
        optional_display, posted_invoice_can_accept_payment, posted_new_cancelled,
        tax_assoc::load_posted_invoice_tax_ids,
    },
    routes::PostedInvoiceCancelGetRouteTag,
    scope::{find_active_posted, find_cancellable_posted, hub_tab_url},
    state::InvoicesState,
    templates::{CancelBulkInvoicePage, CancelInvoicePage, PostedInvoiceDetailPage},
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkCancelQuery {
    #[serde(default)]
    pub ids: Option<String>,
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkCancelForm {
    #[serde(default)]
    pub ids: String,
    #[serde(default, rename = "Reason")]
    pub reason: String,
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

fn bulk_cancel_page(
    ids: &[i64],
    reason: String,
    error: String,
    can_edit: bool,
) -> CancelBulkInvoicePage {
    CancelBulkInvoicePage {
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        count: ids.len(),
        form: CancelInvoiceForm { reason },
        can_edit,
        error,
    }
}

pub async fn detail(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(p) = find_active_posted(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    };

    let tax_ids = load_posted_invoice_tax_ids(&state.db, p.id)
        .await
        .unwrap_or_default();
    let tax_labels = invoice_header_tax_labels(&state.db, &tax_ids).await;
    let customer_name = invoice_customer_name(&state.db, p.customer_id).await;
    let currency = load_journal_entry_currency_format(&state.db, p.journal_entry_id).await;
    let payment_term_rows =
        posted_payment_term_display_rows(&state.db, p.id, currency.minor_unit, &currency.symbol)
            .await;
    let line_rows = posted_invoice_line_display_rows(&state.db, p.id).await;
    let can_edit = require_superuser(&ctx);
    let can_pay = can_edit && posted_invoice_can_accept_payment(&state.db, p.id).await;
    let page = PostedInvoiceDetailPage {
        id: p.id,
        number: p.number,
        reference: optional_display(&p.reference),
        payment_reference: optional_display(&p.payment_reference),
        bank_account: optional_display(&p.bank_account),
        datetime: format_invoice_date(p.datetime, &ctx.timezone),
        delivery_date: {
            let s = format_delivery_date(p.delivery_date);
            if s.is_empty() { "—".to_string() } else { s }
        },
        customer_id: p.customer_id,
        customer_name,
        payment_term_rows,
        tax_labels,
        line_rows,
        journal_entry_id: p.journal_entry_id,
        can_edit,
        can_pay,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn cancel_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if find_cancellable_posted(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    let page = CancelInvoicePage {
        id,
        form: CancelInvoiceForm {
            reason: String::new(),
        },
        can_edit: require_superuser(&ctx),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn cancel_invoice(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<CancelInvoiceForm>,
) -> Response {
    if find_cancellable_posted(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    match posted_new_cancelled(&state.db, id, form.reason, Utc::now()).await {
        Ok(c) => Redirect::to(&format!("/finance-invoices/cancelled/{}/", c.id)).into_response(),
        Err(_) => Redirect::to(&PostedInvoiceCancelGetRouteTag::new(id).url()).into_response(),
    }
}

pub async fn bulk_cancel_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkCancelQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let can_edit = require_superuser(&ctx);
    let page = if ids.is_empty() {
        bulk_cancel_page(
            &ids,
            String::new(),
            "Select at least one posted invoice to cancel.".into(),
            can_edit,
        )
    } else {
        bulk_cancel_page(&ids, String::new(), String::new(), can_edit)
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn bulk_cancel_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkCancelForm>,
) -> Response {
    let ids = parse_bulk_ids(&form.ids);
    let can_edit = require_superuser(&ctx);
    if !can_edit {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    if ids.is_empty() {
        let page = bulk_cancel_page(
            &ids,
            form.reason,
            "Select at least one posted invoice to cancel.".into(),
            can_edit,
        );
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let reason = form.reason.trim().to_string();
    if reason.is_empty() {
        let page = bulk_cancel_page(&ids, form.reason, "Reason is required.".into(), can_edit);
        return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let now = Utc::now();
    for id in &ids {
        if find_cancellable_posted(&state.db, *id).await.is_none() {
            continue;
        }
        if let Err(e) = posted_new_cancelled(&state.db, *id, reason.clone(), now).await {
            tracing::error!(error = %e, id, "failed to bulk-cancel posted invoice");
            let page = bulk_cancel_page(
                &ids,
                reason,
                format!("Failed to cancel posted invoice #{id}: {e}"),
                can_edit,
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }
    Redirect::to(&hub_tab_url("cancelled")).into_response()
}
