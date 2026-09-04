use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};

use sea_orm::EntityTrait;

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{Htmx, html_built_page_or_app_layout},
};

use crate::plugins::finance_common::require_superuser;

use crate::plugins::finance_accounts::scope::load_journal_currency_format;

use crate::plugins::finance_invoices::{
    entities::{
        cancelled_invoice::Entity as CancelledInvoiceEntity,
        posted_invoice::Entity as PostedInvoiceEntity,
    },
    logic::{
        cancelled_new_draft,
        draft_payment_term::cancelled_payment_term_display_rows,
        invoice_line_editor::{
            cancelled_invoice_line_display_rows, invoice_customer_name, invoice_header_tax_labels,
        },
        load_invoice_date_formats, optional_display,
        tax_assoc::load_cancelled_invoice_tax_ids,
    },
    routes::PostedInvoiceDetailRouteTag,
    scope::hub_tab_url,
    state::InvoicesState,
    templates::CancelledInvoiceDetailPage,
};

use crate::plugins::finance_creditnotes::{
    entities::credit_note::Entity as CreditNoteEntity, routes::CreditNoteDetailRouteTag,
};

fn credit_note_display_label(id: i64, date: &str, reason: Option<&str>) -> String {
    if let Some(reason) = reason.map(str::trim).filter(|s| !s.is_empty()) {
        let summary = if reason.len() > 48 {
            format!("{}…", &reason[..45])
        } else {
            reason.to_string()
        };
        format!("#{id} · {date} · {summary}")
    } else {
        format!("#{id} · {date}")
    }
}

fn posted_invoice_display_label(id: i64, number: &str) -> String {
    if number.trim().is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

pub async fn detail(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let cancelled = crate::web::opt_or_log(
        CancelledInvoiceEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    );
    let page = if let Some(c) = cancelled {
        let tax_ids = load_cancelled_invoice_tax_ids(&state.db, c.id)
            .await
            .unwrap_or_default();
        let tax_labels = invoice_header_tax_labels(&state.db, &tax_ids).await;
        let customer_name = invoice_customer_name(&state.db, c.customer_id).await;
        let currency = load_journal_currency_format(&state.db, c.journal_id).await;
        let dates = load_invoice_date_formats(&state.db).await;
        let payment_term_rows = cancelled_payment_term_display_rows(
            &state.db,
            c.id,
            currency.minor_unit,
            &currency.symbol,
            &dates.date,
        )
        .await;
        let line_rows = cancelled_invoice_line_display_rows(&state.db, c.id).await;

        let (posted_invoice_label, posted_invoice_href) = if let Ok(Some(posted)) =
            PostedInvoiceEntity::find_by_id(c.posted_invoice_id)
                .one(&state.db)
                .await
        {
            (
                posted_invoice_display_label(posted.id, &posted.number),
                Some(PostedInvoiceDetailRouteTag::new(posted.id).url()),
            )
        } else {
            (format!("#{}", c.posted_invoice_id), None)
        };

        let (credit_note_label, credit_note_href) = if let Ok(Some(cn)) =
            CreditNoteEntity::find_by_id(c.credit_note_id)
                .one(&state.db)
                .await
        {
            (
                credit_note_display_label(
                    cn.id,
                    &dates.datetime(cn.datetime, &ctx.timezone),
                    cn.reason.as_deref(),
                ),
                Some(CreditNoteDetailRouteTag::new(cn.id).url()),
            )
        } else {
            (format!("Credit note #{}", c.credit_note_id), None)
        };

        CancelledInvoiceDetailPage {
            id: c.id,
            number: c.number,
            reference: optional_display(&c.reference),
            payment_reference: optional_display(&c.payment_reference),
            bank_account: optional_display(&c.bank_account),
            datetime: dates.datetime(c.datetime, &ctx.timezone),
            delivery_date: dates.calendar_or_dash(c.delivery_date),
            customer_id: c.customer_id,
            customer_name,
            payment_term_rows,
            tax_labels,
            line_rows,
            posted_invoice_label,
            posted_invoice_href,
            credit_note_label,
            credit_note_href,
            can_edit: require_superuser(&ctx),
        }
    } else {
        CancelledInvoiceDetailPage {
            id,
            number: "Not found".to_string(),
            reference: String::new(),
            payment_reference: String::new(),
            bank_account: String::new(),
            datetime: String::new(),
            delivery_date: String::new(),
            customer_id: 0,
            customer_name: String::new(),
            payment_term_rows: vec![],
            tax_labels: String::new(),
            line_rows: vec![],
            posted_invoice_label: String::new(),
            posted_invoice_href: None,
            credit_note_label: String::new(),
            credit_note_href: None,
            can_edit: false,
        }
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn new_draft(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/cancelled/").into_response();
    }
    match cancelled_new_draft(&state.db, id, &ctx.timezone).await {
        Ok(d) => Redirect::to(&format!("/finance-invoices/i/{}/", d.id)).into_response(),
        Err(_) => Redirect::to(&format!("/finance-invoices/cancelled/{id}/")).into_response(),
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct BulkNewDraftQuery {
    #[serde(default)]
    pub ids: Option<String>,
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

pub async fn bulk_new_draft(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkNewDraftQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to(&hub_tab_url("cancelled")).into_response();
    }
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return Redirect::to(&hub_tab_url("cancelled")).into_response();
    }
    for id in ids {
        if CancelledInvoiceEntity::find_by_id(id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            continue;
        }
        if let Err(e) = cancelled_new_draft(&state.db, id, &ctx.timezone).await {
            tracing::error!(error = %e, id, "failed to bulk-create draft from cancelled invoice");
            return Redirect::to(&format!("/finance-invoices/cancelled/{id}/")).into_response();
        }
    }
    Redirect::to(&hub_tab_url("drafts")).into_response()
}
