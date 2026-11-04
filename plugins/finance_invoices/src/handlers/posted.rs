use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use maud::Markup;
use sea_orm::{DatabaseConnection, TransactionTrait};

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{CsrfToken, HtmlFormBody};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;

use lariv_plugin_finance_accounts::logic::journal::{
    cascade_delete_preview, collect_journal_entry_cascade, delete_journal_entry_recursive,
};
use lariv_plugin_finance_accounts::scope::load_journal_entry_currency_format;
use lariv_plugin_finance_accounts::source_doc_registry::SourceDocRegistry;

use crate::{
    draft_form_addon::run_draft_invoice_on_new_draft_from_posted,
    forms::CancelInvoiceForm,
    keys::{POSTED_INVOICE_BULK_NEW_DRAFT_MODAL, POSTED_INVOICE_NEW_DRAFT_MODAL},
    logic::{
        delete_draft, delete_quotation,
        draft_payment_term::posted_payment_term_display_rows,
        invoice_line_editor::{
            invoice_customer_name, invoice_header_tax_labels, posted_invoice_line_display_rows,
        },
        load_invoice_date_formats, optional_display, posted_invoice_can_accept_payment,
        posted_new_cancelled, posted_new_draft, quotation_to_draft,
        tax_assoc::load_posted_invoice_tax_ids,
    },
    routes::{
        CancelledInvoiceDetailRouteTag, DraftInvoiceDetailRouteTag,
        PostedInvoiceBulkNewDraftRouteTag, PostedInvoiceCancelGetRouteTag,
        PostedInvoiceDetailRouteTag, PostedInvoiceNewDraftRouteTag, QuotationInvoiceDetailRouteTag,
    },
    scope::{find_active_posted, find_cancellable_posted, hub_tab_url},
    state::InvoicesState,
    templates::{
        CancelBulkInvoicePage, CancelInvoicePage, ConfirmInvoiceTransition,
        PostedInvoiceDetailPage, ResetToDraftDeleteItem, confirm_invoice_transition,
        posted_reset_to_draft_modal,
    },
};

use super::drafts::bulk_transition_post_url;

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
        form: CancelInvoiceForm {
            reason,
            csrf: CsrfToken::current(),
        },
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
    let party = crate::logic::bill_to::load_bill_to(&state.db, p.customer_id).await;
    let customer_name = invoice_customer_name(&state.db, party).await;
    let currency = load_journal_entry_currency_format(&state.db, p.journal_entry_id).await;
    let dates = load_invoice_date_formats(&state.db).await;
    let payment_term_rows = posted_payment_term_display_rows(
        &state.db,
        p.id,
        currency.minor_unit,
        &currency.symbol,
        &dates.date,
    )
    .await;
    let line_rows = posted_invoice_line_display_rows(&state.db, p.id).await;
    let can_edit = lariv_core::components::role_permitted(
        &lariv_plugin_users::role_authorization::roles_for::<crate::routes::FinanceInvoicesMutate>(
        ),
    );
    let can_pay = can_edit && posted_invoice_can_accept_payment(&state.db, p.id).await;
    let page = PostedInvoiceDetailPage {
        id: p.id,
        number: p.number,
        reference: optional_display(&p.reference),
        eway_bill_number: optional_display(&p.eway_bill_number),
        payment_reference: optional_display(&p.payment_reference),
        bank_account: optional_display(&p.bank_account),
        remarks: optional_display(&p.remarks),
        datetime: dates.datetime(p.datetime, &ctx.timezone),
        delivery_date: dates.calendar_or_dash(p.delivery_date),
        bill_to_individual: party.bill_to_individual,
        customer_id: party.party_id(),
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
            csrf: CsrfToken::current(),
        },
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinanceInvoicesMutate,
            >(),
        ),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn cancel_invoice(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<CancelInvoiceForm>,
) -> Response {
    if find_cancellable_posted(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    match posted_new_cancelled(&state.db, id, form.reason, Utc::now()).await {
        Ok(c) => Redirect::to(&CancelledInvoiceDetailRouteTag::new(c.id).url()).into_response(),
        Err(_) => Redirect::to(&PostedInvoiceCancelGetRouteTag::new(id).url()).into_response(),
    }
}

pub async fn new_draft_confirm_get(RequireAuth(_ctx): RequireAuth, Path(id): Path<i64>) -> Markup {
    let post_url = PostedInvoiceNewDraftRouteTag::new(id).path();
    confirm_invoice_transition(ConfirmInvoiceTransition {
        modal_uid: POSTED_INVOICE_NEW_DRAFT_MODAL.id(),
        title: "New quotation",
        message: "Create a new quotation from this posted invoice? The posted record will be unchanged.",
        submit_label: "Create quotation",
        post_url: &post_url,
        can_submit: true,
    })
}

pub async fn new_draft(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if find_active_posted(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    match posted_new_draft(&state.db, id, &ctx.timezone).await {
        Ok(d) => Redirect::to(&QuotationInvoiceDetailRouteTag::new(d.id).url()).into_response(),
        Err(e) => {
            tracing::error!(error = %e, id, "failed to create draft from posted invoice");
            Redirect::to(&PostedInvoiceDetailRouteTag::new(id).url()).into_response()
        }
    }
}

pub async fn reset_draft_get(
    Cap(state): Cap<InvoicesState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    let Some(posted) = find_active_posted(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    };
    reset_draft_modal(&state.db, &source_docs, id, posted.journal_entry_id, None)
        .await
        .into_response()
}

pub async fn reset_draft_post(
    Cap(state): Cap<InvoicesState>,
    Cap(source_docs): Cap<SourceDocRegistry>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(posted) = find_active_posted(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    };
    let journal_entry_id = posted.journal_entry_id;
    if journal_entry_id <= 0 {
        return reset_draft_modal(
            &state.db,
            &source_docs,
            id,
            journal_entry_id,
            Some("This posted invoice has no journal entry.".into()),
        )
        .await
        .into_response();
    }
    let draft_id = match copy_posted_invoice_to_draft(&state.db, id, &ctx.timezone).await {
        Ok(draft_id) => draft_id,
        Err(e) => {
            tracing::error!(error = %e, id, "failed to create draft while resetting posted invoice");
            return reset_draft_modal(&state.db, &source_docs, id, journal_entry_id, Some(e))
                .await
                .into_response();
        }
    };
    match delete_journal_entry_recursive(&state.db, journal_entry_id).await {
        Ok(()) => htmx.redirect(&DraftInvoiceDetailRouteTag::new(draft_id).url()),
        Err(e) => {
            tracing::error!(
                error = %e,
                id,
                journal_entry_id,
                draft_id,
                "failed to delete journal entry while resetting posted invoice"
            );
            let error = match delete_draft(&state.db, draft_id).await {
                Ok(()) => e.to_string(),
                Err(rollback) => {
                    tracing::error!(
                        error = %rollback,
                        draft_id,
                        "failed to roll back draft after reset-to-draft delete failed"
                    );
                    format!(
                        "{e} (the new draft invoice #{draft_id} could not be removed: {rollback})"
                    )
                }
            };
            reset_draft_modal(&state.db, &source_docs, id, journal_entry_id, Some(error))
                .await
                .into_response()
        }
    }
}

/// Copy a posted invoice into a draft: New Draft, then the quotation-to-draft copy.
///
/// The intermediate quotation is removed. Addon data on the posted invoice is copied
/// onto the draft before the posted row is deleted.
async fn copy_posted_invoice_to_draft(
    db: &DatabaseConnection,
    posted_id: i64,
    tz: &str,
) -> Result<i64, String> {
    let quotation = posted_new_draft(db, posted_id, tz).await?;
    let draft = match quotation_to_draft(db, quotation.id).await {
        Ok(draft) => draft,
        Err(e) => {
            return Err(rollback_quotation(db, quotation.id, e).await);
        }
    };
    if let Err(e) = delete_quotation(db, quotation.id).await {
        return Err(rollback_draft(db, draft.id, e).await);
    }
    let txn = match db.begin().await {
        Ok(txn) => txn,
        Err(e) => return Err(rollback_draft(db, draft.id, e.to_string()).await),
    };
    if let Err(e) = run_draft_invoice_on_new_draft_from_posted(&txn, posted_id, draft.id).await {
        if let Err(rollback) = txn.rollback().await {
            tracing::error!(error = %rollback, "failed to roll back draft addon copy");
        }
        return Err(rollback_draft(db, draft.id, e).await);
    }
    if let Err(e) = txn.commit().await {
        return Err(rollback_draft(db, draft.id, e.to_string()).await);
    }
    Ok(draft.id)
}

async fn rollback_quotation(db: &DatabaseConnection, quotation_id: i64, error: String) -> String {
    match delete_quotation(db, quotation_id).await {
        Ok(()) => error,
        Err(rollback) => {
            tracing::error!(
                error = %rollback,
                quotation_id,
                "failed to remove quotation created during reset to draft"
            );
            format!("{error} (quotation #{quotation_id} could not be removed: {rollback})")
        }
    }
}

async fn rollback_draft(db: &DatabaseConnection, draft_id: i64, error: String) -> String {
    match delete_draft(db, draft_id).await {
        Ok(()) => error,
        Err(rollback) => {
            tracing::error!(
                error = %rollback,
                draft_id,
                "failed to remove draft created during reset to draft"
            );
            format!("{error} (draft invoice #{draft_id} could not be removed: {rollback})")
        }
    }
}

async fn reset_draft_modal(
    db: &DatabaseConnection,
    registry: &SourceDocRegistry,
    id: i64,
    journal_entry_id: i64,
    error: Option<String>,
) -> Markup {
    if journal_entry_id <= 0 {
        return posted_reset_to_draft_modal(
            id,
            &[],
            Some(
                error
                    .as_deref()
                    .unwrap_or("This posted invoice has no journal entry."),
            ),
            false,
        );
    }
    let (items, preview_error) = match collect_journal_entry_cascade(db, journal_entry_id).await {
        Ok(graph) => match cascade_delete_preview(db, registry, &graph).await {
            Ok(items) => (
                items
                    .into_iter()
                    .map(|item| ResetToDraftDeleteItem {
                        kind: item.kind,
                        label: item.label,
                        url: item.url,
                    })
                    .collect(),
                None,
            ),
            Err(e) => (Vec::new(), Some(e.to_string())),
        },
        Err(e) => (Vec::new(), Some(e.to_string())),
    };
    let can_submit = preview_error.is_none();
    posted_reset_to_draft_modal(
        id,
        &items,
        preview_error.as_deref().or(error.as_deref()),
        can_submit,
    )
}

pub async fn bulk_new_draft_confirm_get(
    RequireAuth(_ctx): RequireAuth,
    Query(q): Query<BulkCancelQuery>,
) -> Markup {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let (message, post_url, can_submit) = if ids.is_empty() {
        (
            "Select at least one posted invoice.".to_string(),
            PostedInvoiceBulkNewDraftRouteTag.path(),
            false,
        )
    } else {
        (
            "Create new quotations from the selected posted invoices? The posted records will be unchanged.".to_string(),
            bulk_transition_post_url(&PostedInvoiceBulkNewDraftRouteTag.path(), &ids),
            true,
        )
    };
    confirm_invoice_transition(ConfirmInvoiceTransition {
        modal_uid: POSTED_INVOICE_BULK_NEW_DRAFT_MODAL.id(),
        title: "New quotations from posted invoices",
        message: &message,
        submit_label: "Create quotations",
        post_url: &post_url,
        can_submit,
    })
}

pub async fn bulk_new_draft(
    Cap(state): Cap<InvoicesState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkCancelQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    for id in ids {
        if find_active_posted(&state.db, id).await.is_none() {
            continue;
        }
        if let Err(e) = posted_new_draft(&state.db, id, &ctx.timezone).await {
            tracing::error!(error = %e, id, "failed to bulk-create draft from posted invoice");
            return Redirect::to(&PostedInvoiceDetailRouteTag::new(id).url()).into_response();
        }
    }
    Redirect::to(&hub_tab_url("quotations")).into_response()
}

pub async fn bulk_cancel_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkCancelQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let can_edit = lariv_core::components::role_permitted(
        &lariv_plugin_users::role_authorization::roles_for::<crate::routes::FinanceInvoicesMutate>(
        ),
    );
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
    let can_edit = lariv_core::components::role_permitted(
        &lariv_plugin_users::role_authorization::roles_for::<crate::routes::FinanceInvoicesMutate>(
        ),
    );
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
