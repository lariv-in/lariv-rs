use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Response},
};
use chrono::Utc;
use maud::{Markup, html};
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait};

use lariv_core::components::{ManyToManyItem, SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{CsrfToken, HtmlFormBody};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_with_slots, respond_create_modal_done};
use lariv_plugin_finance_accounts::journal_lines_editor::input_journal_lines;
use lariv_plugin_finance_accounts::logic::custom_source_doc::{
    ensure_posting_accounts, parse_submitted_journal_lines,
};
use lariv_plugin_finance_accounts::scope::load_journal_currency_format;
use lariv_plugin_finance_common::decimal::decimal_display_currency;
use lariv_plugin_finance_payments::logic::parse_payment_amount;
use lariv_plugin_finance_taxes::scope::{load_taxes_by_ids, tax_label};
use lariv_plugin_users::middleware::RequireAuth;

use crate::{
    entities::PostedInvoiceEntity,
    forms::InvoicePaymentForm,
    handlers::ModalNameQuery,
    keys::INVOICE_PAY_MODAL,
    logic::{
        PayPostedInvoiceInput, parse_invoice_datetime, pay_posted_invoice,
        posted_invoice_open_balance, suggest_invoice_payment_lines,
    },
    routes::{PaidInvoiceDetailRouteTag, PartiallyPaidInvoiceDetailRouteTag},
    state::InvoicesState,
    templates::InvoicePayModalPage,
};

#[derive(Debug, serde::Deserialize, Default)]
pub struct PayQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
}

async fn tax_items(db: &DatabaseConnection, tax_ids: &[i64]) -> Vec<ManyToManyItem> {
    let taxes = load_taxes_by_ids(db, tax_ids).await.unwrap_or_default();
    taxes
        .iter()
        .map(|t| ManyToManyItem::new(t.id.to_string(), tax_label(t)))
        .collect()
}

async fn pay_modal(
    db: &DatabaseConnection,
    q: &PayQuery,
    form: InvoicePaymentForm,
    error: String,
) -> InvoicePayModalPage {
    let tax_items = tax_items(db, &form.taxes).await;
    InvoicePayModalPage {
        form_name: q.modal.form_name(),
        refresh_table: q.modal.refresh_table(),
        form,
        tax_items,
        error,
    }
}

pub async fn pay_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
    Query(q): Query<PayQuery>,
) -> Response {
    let open = posted_invoice_open_balance(&state.db, id)
        .await
        .unwrap_or(Decimal::ZERO);
    let amount = if open > Decimal::ZERO {
        let journal_id = lariv_core::web::opt_or_log(
            PostedInvoiceEntity::find_by_id(id).one(&state.db).await,
            "find posted invoice for pay amount",
        )
        .map(|row| row.journal_id)
        .unwrap_or(0);
        let currency = load_journal_currency_format(&state.db, journal_id).await;
        decimal_display_currency(open, currency.minor_unit, "")
    } else {
        String::new()
    };
    let (journal_lines, error) = match parse_payment_amount(&amount) {
        Ok(parsed) => match suggest_invoice_payment_lines(&state.db, id, parsed, &[]).await {
            Ok(lines) => (lines, String::new()),
            Err(error) => (String::new(), error),
        },
        _ => (String::new(), String::new()),
    };
    let page = pay_modal(
        &state.db,
        &q,
        InvoicePaymentForm {
            posted_invoice_id: id,
            amount,
            datetime: ctx.datetime_local_input(Utc::now()).into_string(),
            taxes: vec![],
            journal_lines,
            csrf: CsrfToken::current(),
        },
        error,
    )
    .await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn pay_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
    Query(q): Query<PayQuery>,
    HtmlFormBody(mut form): HtmlFormBody<InvoicePaymentForm>,
) -> Response {
    form.posted_invoice_id = id;
    let amount = match parse_payment_amount(&form.amount) {
        Ok(a) => a,
        Err(e) => {
            let page = pay_modal(&state.db, &q, form, e).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    let submitted = match parse_submitted_journal_lines(&form.journal_lines) {
        Ok(lines) => lines,
        Err(error) => {
            let page = pay_modal(&state.db, &q, form, error).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };
    if let Err(error) = ensure_posting_accounts(&state.db, &submitted.lines).await {
        let page = pay_modal(&state.db, &q, form, error).await;
        return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
            .into_response();
    }
    let input = PayPostedInvoiceInput {
        posted_invoice_id: id,
        amount,
        datetime: parse_invoice_datetime(&form.datetime, &ctx.timezone),
        withholding_tax_ids: form.taxes.clone(),
        journal_lines: Some(submitted.lines),
    };
    match pay_posted_invoice(&state.db, input).await {
        Ok(result) => {
            let detail_url = if result.is_full {
                PaidInvoiceDetailRouteTag::new(result.settlement_id).url()
            } else {
                PartiallyPaidInvoiceDetailRouteTag::new(result.settlement_id).url()
            };
            respond_create_modal_done(
                &htmx,
                INVOICE_PAY_MODAL,
                &q.modal.refresh_table(),
                &detail_url,
            )
        }
        Err(e) => {
            let page = pay_modal(&state.db, &q, form, e).await;
            html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
        }
    }
}

pub async fn fill_journal_lines(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    HtmlFormBody(form): HtmlFormBody<InvoicePaymentForm>,
) -> Markup {
    let suggested = if form.posted_invoice_id <= 0 {
        Err("posted invoice is required".to_string())
    } else {
        match parse_payment_amount(&form.amount) {
            Ok(amount) => {
                suggest_invoice_payment_lines(
                    &state.db,
                    form.posted_invoice_id,
                    amount,
                    &form.taxes,
                )
                .await
            }
            Err(error) => Err(error),
        }
    };
    match suggested {
        Ok(lines) => input_journal_lines("JournalLines", "Journal lines", &lines),
        Err(error) => html! {
            p class="text-error mb-2" { (error) }
            (input_journal_lines("JournalLines", "Journal lines", &form.journal_lines))
        },
    }
}
