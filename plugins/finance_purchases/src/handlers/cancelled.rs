use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};

use sea_orm::EntityTrait;

use lariv_core::components::{SharedChromeFolder, SlotCtx};
use lariv_core::http::Cap;
use lariv_core::web::{Htmx, html_built_page_or_app_layout};
use lariv_plugin_users::middleware::RequireAuth;

use lariv_plugin_finance_accounts::routes::JournalEntryDetailRouteTag;
use lariv_plugin_finance_accounts::scope::load_journal_currency_format;

use crate::{
    entities::{
        cancelled_purchase::Entity as CancelledPurchaseEntity,
        posted_purchase::Entity as PostedPurchaseEntity,
    },
    logic::{
        cancelled_new_draft,
        draft_payment_term::cancelled_payment_term_display_rows,
        purchase_line_editor::{
            cancelled_purchase_line_display_rows, purchase_vendor_name, purchase_header_tax_labels,
        },
        load_purchase_date_formats, optional_display,
        tax_assoc::load_cancelled_purchase_tax_ids,
    },
    routes::{
        CancelledPurchaseDetailRouteTag, DraftPurchaseDetailRouteTag, PostedPurchaseDetailRouteTag,
    },
    scope::hub_tab_url,
    state::PurchasesState,
    templates::CancelledPurchaseDetailPage,
};

fn posted_purchase_display_label(id: i64, number: &str) -> String {
    if number.trim().is_empty() {
        format!("#{id}")
    } else {
        number.to_string()
    }
}

pub async fn detail(
    Cap(state): Cap<PurchasesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Path(id): Path<i64>,
) -> Response {
    let cancelled = lariv_core::web::opt_or_log(
        CancelledPurchaseEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    );
    let page = if let Some(c) = cancelled {
        let tax_ids = load_cancelled_purchase_tax_ids(&state.db, c.id)
            .await
            .unwrap_or_default();
        let tax_labels = purchase_header_tax_labels(&state.db, &tax_ids).await;
        let party = crate::logic::BillTo::new(
            c.vendor_is_individual,
            c.vendor_contact_id,
            c.vendor_company_id,
        );
        let vendor_name = purchase_vendor_name(&state.db, party).await;
        let currency = load_journal_currency_format(&state.db, c.journal_id).await;
        let dates = load_purchase_date_formats(&state.db).await;
        let payment_term_rows = cancelled_payment_term_display_rows(
            &state.db,
            c.id,
            currency.minor_unit,
            &currency.symbol,
            &dates.date,
        )
        .await;
        let line_rows = cancelled_purchase_line_display_rows(&state.db, c.id).await;

        let (posted_purchase_label, posted_purchase_href) = if let Ok(Some(posted)) =
            PostedPurchaseEntity::find_by_id(c.posted_purchase_id)
                .one(&state.db)
                .await
        {
            (
                posted_purchase_display_label(posted.id, &posted.number),
                Some(PostedPurchaseDetailRouteTag::new(posted.id).url()),
            )
        } else {
            (format!("#{}", c.posted_purchase_id), None)
        };

        let reversal_label = format!("Journal entry #{}", c.reversed_journal_entry_id);
        let reversal_href = Some(JournalEntryDetailRouteTag::new(c.reversed_journal_entry_id).url());

        CancelledPurchaseDetailPage {
            id: c.id,
            number: c.number,
            reference: optional_display(&c.reference),
            payment_reference: optional_display(&c.payment_reference),
            bank_account: optional_display(&c.bank_account),
            remarks: optional_display(&c.remarks),
            datetime: dates.datetime(c.datetime, &ctx.timezone),
            delivery_date: dates.calendar_or_dash(c.delivery_date),
            vendor_is_individual: party.vendor_is_individual,
            vendor_id: party.party_id(),
            vendor_name,
            payment_term_rows,
            tax_labels,
            line_rows,
            posted_purchase_label,
            posted_purchase_href,
            reason: optional_display(&c.reason),
            reversal_label,
            reversal_href,
            can_edit: lariv_core::components::role_permitted(
                &lariv_plugin_users::role_authorization::roles_for::<
                    crate::routes::FinancePurchasesMutate,
                >(),
            ),
        }
    } else {
        CancelledPurchaseDetailPage {
            id,
            number: "Not found".to_string(),
            reference: String::new(),
            payment_reference: String::new(),
            bank_account: String::new(),
            remarks: String::new(),
            datetime: String::new(),
            delivery_date: String::new(),
            vendor_is_individual: false,
            vendor_id: 0,
            vendor_name: String::new(),
            payment_term_rows: vec![],
            tax_labels: String::new(),
            line_rows: vec![],
            posted_purchase_label: String::new(),
            posted_purchase_href: None,
            reason: String::new(),
            reversal_label: String::new(),
            reversal_href: None,
            can_edit: false,
        }
    };
    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn new_draft(
    Cap(state): Cap<PurchasesState>,
    RequireAuth(ctx): RequireAuth,
    Path(id): Path<i64>,
) -> Response {
    match cancelled_new_draft(&state.db, id, &ctx.timezone).await {
        Ok(d) => Redirect::to(&DraftPurchaseDetailRouteTag::new(d.id).url()).into_response(),
        Err(_) => Redirect::to(&CancelledPurchaseDetailRouteTag::new(id).url()).into_response(),
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
    Cap(state): Cap<PurchasesState>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BulkNewDraftQuery>,
) -> Response {
    let ids = parse_bulk_ids(q.ids.as_deref().unwrap_or(""));
    if ids.is_empty() {
        return Redirect::to(&hub_tab_url("cancelled")).into_response();
    }
    for id in ids {
        if CancelledPurchaseEntity::find_by_id(id)
            .one(&state.db)
            .await
            .ok()
            .flatten()
            .is_none()
        {
            continue;
        }
        if let Err(e) = cancelled_new_draft(&state.db, id, &ctx.timezone).await {
            tracing::error!(error = %e, id, "failed to bulk-create draft from cancelled purchase");
            return Redirect::to(&CancelledPurchaseDetailRouteTag::new(id).url()).into_response();
        }
    }
    Redirect::to(&hub_tab_url("drafts")).into_response()
}
