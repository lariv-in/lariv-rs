use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::html_form::{CsrfToken, HtmlFormBody};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout, html_built_page_with_slots};
use lariv_plugin_users::middleware::RequireAuth;

use lariv_plugin_finance_accounts::scope::load_journal_entry_currency_format;

use crate::{
    forms::CancelPurchaseForm,
    logic::{
        draft_payment_term::posted_payment_term_display_rows,
        purchase_line_editor::{
            purchase_vendor_name, purchase_header_tax_labels, posted_purchase_line_display_rows,
        },
        load_purchase_date_formats, optional_display, posted_new_cancelled,
        tax_assoc::load_posted_purchase_tax_ids,
    },
    routes::{CancelledPurchaseDetailRouteTag, PostedPurchaseCancelGetRouteTag},
    scope::{find_active_posted, find_cancellable_posted, hub_tab_url},
    state::PurchasesState,
    templates::{CancelBulkPurchasePage, CancelPurchasePage, PostedPurchaseDetailPage},
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
) -> CancelBulkPurchasePage {
    CancelBulkPurchasePage {
        ids: ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(","),
        count: ids.len(),
        form: CancelPurchaseForm {
            reason,
            csrf: CsrfToken::current(),
        },
        can_edit,
        error,
    }
}

pub async fn detail(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let Some(p) = find_active_posted(&state.db, id).await else {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    };

    let tax_ids = load_posted_purchase_tax_ids(&state.db, p.id)
        .await
        .unwrap_or_default();
    let tax_labels = purchase_header_tax_labels(&state.db, &tax_ids).await;
    let party = crate::logic::BillTo::new(
        p.vendor_is_individual,
        p.vendor_contact_id,
        p.vendor_company_id,
    );
    let vendor_name = purchase_vendor_name(&state.db, party).await;
    let currency = load_journal_entry_currency_format(&state.db, p.journal_entry_id).await;
    let dates = load_purchase_date_formats(&state.db).await;
    let payment_term_rows = posted_payment_term_display_rows(
        &state.db,
        p.id,
        currency.minor_unit,
        &currency.symbol,
        &dates.date,
    )
    .await;
    let line_rows = posted_purchase_line_display_rows(&state.db, p.id).await;
    let can_edit = lariv_core::components::role_permitted(
        &lariv_plugin_users::role_authorization::roles_for::<crate::routes::FinancePurchasesMutate>(
        ),
    );
    let page = PostedPurchaseDetailPage {
        id: p.id,
        number: p.number,
        reference: optional_display(&p.reference),
        payment_reference: optional_display(&p.payment_reference),
        bank_account: optional_display(&p.bank_account),
        remarks: optional_display(&p.remarks),
        datetime: dates.datetime(p.datetime, &ctx.timezone),
        delivery_date: dates.calendar_or_dash(p.delivery_date),
        vendor_is_individual: party.vendor_is_individual,
        vendor_id: party.party_id(),
        vendor_name,
        payment_term_rows,
        tax_labels,
        line_rows,
        journal_entry_id: p.journal_entry_id,
        can_edit,
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn cancel_get(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    if find_cancellable_posted(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    let page = CancelPurchasePage {
        id,
        form: CancelPurchaseForm {
            reason: String::new(),
            csrf: CsrfToken::current(),
        },
        can_edit: lariv_core::components::role_permitted(
            &lariv_plugin_users::role_authorization::roles_for::<
                crate::routes::FinancePurchasesMutate,
            >(),
        ),
    };
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn cancel_purchase(
    Cap(state): Cap<PurchasesState>,
    RequireAuth(_ctx): RequireAuth,
    Path(id): Path<i64>,
    HtmlFormBody(form): HtmlFormBody<CancelPurchaseForm>,
) -> Response {
    if find_cancellable_posted(&state.db, id).await.is_none() {
        return Redirect::to(&hub_tab_url("posted")).into_response();
    }
    match posted_new_cancelled(&state.db, id, form.reason, Utc::now()).await {
        Ok(c) => Redirect::to(&CancelledPurchaseDetailRouteTag::new(c.id).url()).into_response(),
        Err(_) => Redirect::to(&PostedPurchaseCancelGetRouteTag::new(id).url()).into_response(),
    }
}

pub async fn bulk_cancel_get(
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BulkCancelQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    let can_edit = lariv_core::components::role_permitted(
        &lariv_plugin_users::role_authorization::roles_for::<crate::routes::FinancePurchasesMutate>(
        ),
    );
    let page = if ids.is_empty() {
        bulk_cancel_page(
            &ids,
            String::new(),
            "Select at least one posted purchase to cancel.".into(),
            can_edit,
        )
    } else {
        bulk_cancel_page(&ids, String::new(), String::new(), can_edit)
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn bulk_cancel_post(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    HtmlFormBody(form): HtmlFormBody<BulkCancelForm>,
) -> Response {
    let ids = parse_bulk_ids(&form.ids);
    let can_edit = lariv_core::components::role_permitted(
        &lariv_plugin_users::role_authorization::roles_for::<crate::routes::FinancePurchasesMutate>(
        ),
    );
    if ids.is_empty() {
        let page = bulk_cancel_page(
            &ids,
            form.reason,
            "Select at least one posted purchase to cancel.".into(),
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
            tracing::error!(error = %e, id, "failed to bulk-cancel posted purchase");
            let page = bulk_cancel_page(
                &ids,
                reason,
                format!("Failed to cancel posted purchase #{id}: {e}"),
                can_edit,
            );
            return html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    }
    Redirect::to(&hub_tab_url("cancelled")).into_response()
}
