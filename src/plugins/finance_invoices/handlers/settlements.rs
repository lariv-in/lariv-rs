use axum::{
    extract::Path,
    response::{IntoResponse, Redirect, Response},
};
use sea_orm::EntityTrait;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::finance_accounts::scope::load_journal_entry_currency_format;
use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_invoices::{
    entities::{payment::Entity as PaymentEntity, posted_invoice::Entity as PostedInvoiceEntity},
    logic::{
        draft_payment_term::posted_payment_term_display_rows,
        invoice_line_editor::{
            invoice_customer_name, invoice_header_tax_labels, posted_invoice_line_display_rows,
        },
        load_invoice_date_formats, optional_display, posted_invoice_can_accept_payment,
        tax_assoc::load_posted_invoice_tax_ids,
    },
    scope::{find_active_paid, find_active_partial, hub_tab_url},
    state::InvoicesState,
    templates::{PaidInvoiceDetailPage, PartiallyPaidInvoiceDetailPage, SettlementDetailContext},
};

async fn load_settlement_context(
    db: &sea_orm::DatabaseConnection,
    settlement_id: i64,
    payment_id: i64,
    posted_invoice_id: i64,
    prior_partially_paid_invoice_id: Option<i64>,
    tz: &str,
) -> Option<SettlementDetailContext> {
    let posted = crate::web::opt_or_log(
        PostedInvoiceEntity::find_by_id(posted_invoice_id)
            .one(db)
            .await,
        "find by id",
    )?;
    let payment = crate::web::opt_or_log(
        PaymentEntity::find_by_id(payment_id).one(db).await,
        "find by id",
    )?;
    let tax_ids = load_posted_invoice_tax_ids(db, posted.id)
        .await
        .unwrap_or_default();
    let tax_labels = invoice_header_tax_labels(db, &tax_ids).await;
    let customer_name = invoice_customer_name(db, posted.customer_id).await;
    let currency = load_journal_entry_currency_format(db, posted.journal_entry_id).await;
    let dates = load_invoice_date_formats(db).await;
    let payment_term_rows = posted_payment_term_display_rows(
        db,
        posted.id,
        currency.minor_unit,
        &currency.symbol,
        &dates.date,
    )
    .await;
    let line_rows = posted_invoice_line_display_rows(db, posted.id).await;
    let currency = load_journal_entry_currency_format(db, payment.journal_entry_id).await;
    let payment_amount = currency.display(payment.amount);
    let payment_label = format!("#{} · {payment_amount}", payment.id);
    let payment_href = format!("/finance-invoices/payments/{}/", payment.id);
    let prior_partial_label = prior_partially_paid_invoice_id
        .filter(|id| *id > 0)
        .map(|id| format!("#{id}"));
    let prior_partial_href = prior_partially_paid_invoice_id
        .filter(|id| *id > 0)
        .map(|id| format!("/finance-invoices/partial/{id}/"));
    Some(SettlementDetailContext {
        settlement_id,
        posted_invoice_id: posted.id,
        number: posted.number,
        reference: optional_display(&posted.reference),
        payment_reference: optional_display(&posted.payment_reference),
        bank_account: optional_display(&posted.bank_account),
        datetime: dates.datetime(posted.datetime, tz),
        posted_at: posted.posted_at.map(|t| dates.datetime(t, tz)),
        customer_id: posted.customer_id,
        customer_name,
        payment_term_rows,
        tax_labels,
        line_rows,
        journal_entry_id: posted.journal_entry_id,
        payment_id: payment.id,
        payment_label,
        payment_href,
        payment_datetime: dates.datetime(payment.datetime, tz),
        prior_partial_label,
        prior_partial_href,
    })
}

pub async fn paid_detail(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(paid) = find_active_paid(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("paid")).into_response();
    };
    let can_edit = require_superuser(&ctx);
    let Some(ctx_data) = load_settlement_context(
        &state.db,
        paid.id,
        paid.payment_id,
        paid.posted_invoice_id,
        paid.prior_partially_paid_invoice_id,
        &ctx.timezone,
    )
    .await
    else {
        return Redirect::to(&hub_tab_url("paid")).into_response();
    };
    let page = PaidInvoiceDetailPage {
        ctx: ctx_data,
        can_edit,
        can_pay: false,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn partial_detail(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(partial) = find_active_partial(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("partial")).into_response();
    };
    let can_edit = require_superuser(&ctx);
    let Some(ctx_data) = load_settlement_context(
        &state.db,
        partial.id,
        partial.payment_id,
        partial.posted_invoice_id,
        partial.prior_partially_paid_invoice_id,
        &ctx.timezone,
    )
    .await
    else {
        return Redirect::to(&hub_tab_url("partial")).into_response();
    };
    let can_pay =
        can_edit && posted_invoice_can_accept_payment(&state.db, partial.posted_invoice_id).await;
    let page = PartiallyPaidInvoiceDetailPage {
        ctx: ctx_data,
        can_edit,
        can_pay,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
