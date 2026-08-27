use axum::{
    Form,
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
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
    templates::{CancelInvoicePage, PostedInvoiceDetailPage},
};

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
    let payment_term_rows = posted_payment_term_display_rows(
        &state.db,
        p.id,
        currency.minor_unit,
        &currency.symbol,
    )
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
            if s.is_empty() {
                "—".to_string()
            } else {
                s
            }
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
    Form(form): Form<CancelInvoiceForm>,
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
