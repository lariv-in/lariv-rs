use std::collections::HashMap;

use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Redirect, Response},
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::{
    components::{SharedChromeFolder, SlotCtx},
    html_form::HtmlFormBody,
    http::Cap,
    plugins::users::middleware::RequireAuth,
    web::{
        html_built_page_or_app_layout, html_built_page_with_slots, respond_create_modal_done, Htmx,
    },
};

use crate::plugins::customer::entities::customer::{self, Entity as CustomerEntity};
use crate::plugins::finance_accounts::scope::{
    load_account_parent_label, load_journal_currency_format, load_journal_entry_currency_format,
};
use crate::plugins::finance_common::require_superuser;
use crate::plugins::finance_taxes::scope::{load_all_taxes, load_taxes_by_ids, tax_label};

use crate::plugins::finance_invoices::{
    entities::{
        payment::{self, Entity as PaymentEntity},
        payment_batch::Entity as PaymentBatchEntity,
        posted_invoice::{self, Entity as PostedInvoiceEntity},
    },
    forms::PaymentBatchForm,
    keys::PaymentBatchCreateModalKey,
    logic::{
        create_payment_batch, parse_batch_allocations_json, parse_invoice_datetime,
        posted_invoice_open_balance, CreatePaymentBatchInput,
    },
    routes::{PaymentBatchDetailRouteTag, PaymentDetailRouteTag, PostedInvoiceDetailRouteTag},
    scope::sql_posted_not_cancelled,
    state::InvoicesState,
    templates::{
        PaymentBatchAllocationRow, PaymentBatchCreateModalPage, PaymentBatchDetailPage,
        PaymentBatchPaymentRow,
    },
};

use super::ModalNameQuery;

#[derive(Debug, serde::Deserialize, Default)]
pub struct BatchCreateQuery {
    #[serde(flatten)]
    pub modal: ModalNameQuery,
    #[serde(default, rename = "PostedInvoiceIDs")]
    pub posted_invoice_ids: Option<String>,
}

fn parse_posted_invoice_ids(raw: &str) -> Vec<i64> {
    raw.split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect()
}

fn posted_invoice_display_label(id: i64, number: &str) -> String {
    if number.trim().is_empty() {
        format!("#{id}")
    } else {
        format!("{number} (#{id})")
    }
}

async fn build_allocations_json(
    db: &sea_orm::DatabaseConnection,
    posted_ids: &[i64],
) -> Result<String, String> {
    if posted_ids.is_empty() {
        return Err("select at least one posted invoice".to_string());
    }

    let models = PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Id.is_in(posted_ids.to_vec()))
        .filter(crate::plugins::finance_invoices::scope::sql_posted_not_fully_paid())
        .filter(crate::plugins::finance_invoices::scope::sql_posted_not_partially_paid())
        .filter(sql_posted_not_cancelled())
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    if models.len() != posted_ids.len() {
        return Err("one or more selected invoices are unavailable".to_string());
    }

    let model_by_id: HashMap<i64, posted_invoice::Model> =
        models.iter().map(|m| (m.id, m.clone())).collect();

    let customer_ids: Vec<i64> = models.iter().map(|m| m.customer_id).collect();
    let customers = if customer_ids.is_empty() {
        HashMap::new()
    } else {
        CustomerEntity::find()
            .filter(customer::Column::Id.is_in(customer_ids))
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect()
    };

    let mut rows = Vec::with_capacity(posted_ids.len());
    for id in posted_ids {
        let inv = model_by_id
            .get(id)
            .ok_or_else(|| format!("posted invoice #{id} not found"))?;
        let open = posted_invoice_open_balance(db, inv.id).await?;
        if open <= Decimal::ZERO {
            return Err(format!(
                "invoice {} has no open balance",
                posted_invoice_display_label(inv.id, &inv.number)
            ));
        }
        let currency = load_journal_currency_format(db, inv.journal_id).await;
        rows.push(PaymentBatchAllocationRow {
            posted_invoice_id: inv.id,
            // Form input amount: plain number.
            amount: crate::plugins::finance_common::decimal::decimal_display(open),
            tax_ids: vec![],
            invoice_number: posted_invoice_display_label(inv.id, &inv.number),
            customer_name: customers
                .get(&inv.customer_id)
                .cloned()
                .unwrap_or_else(|| "—".into()),
            open_balance: currency.display(open),
        });
    }

    serde_json::to_string(&rows).map_err(|e| e.to_string())
}

async fn tax_editor_context(
    db: &sea_orm::DatabaseConnection,
) -> (
    serde_json::Map<String, serde_json::Value>,
    Vec<serde_json::Value>,
) {
    let taxes = load_all_taxes(db).await.unwrap_or_default();
    let mut tax_pct = serde_json::Map::new();
    let mut all_taxes = Vec::new();
    for t in &taxes {
        tax_pct.insert(
            t.id.to_string(),
            serde_json::Value::String(t.percentage.to_string()),
        );
        all_taxes.push(serde_json::json!({
            "id": t.id,
            "name": t.name,
        }));
    }
    (tax_pct, all_taxes)
}

async fn batch_allocations_preview(db: &sea_orm::DatabaseConnection) -> String {
    let (tax_pct, all_taxes) = tax_editor_context(db).await;
    serde_json::json!({
        "tax_pct_by_id": tax_pct,
        "all_taxes": all_taxes,
    })
    .to_string()
}

async fn enrich_allocations_json(
    db: &sea_orm::DatabaseConnection,
    allocations_json: &str,
) -> String {
    #[derive(serde::Deserialize)]
    struct Row {
        posted_invoice_id: i64,
        amount: String,
        #[serde(default)]
        tax_ids: Vec<i64>,
        #[serde(default)]
        invoice_number: String,
        #[serde(default)]
        customer_name: String,
        #[serde(default)]
        open_balance: String,
    }

    let Ok(mut rows) = serde_json::from_str::<Vec<Row>>(allocations_json) else {
        return allocations_json.to_string();
    };
    if rows.is_empty() {
        return allocations_json.to_string();
    }

    let posted_ids: Vec<i64> = rows.iter().map(|r| r.posted_invoice_id).collect();
    let models = PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Id.is_in(posted_ids))
        .all(db)
        .await
        .unwrap_or_default();
    let model_by_id: HashMap<i64, posted_invoice::Model> =
        models.iter().map(|m| (m.id, m.clone())).collect();

    let customer_ids: Vec<i64> = models.iter().map(|m| m.customer_id).collect();
    let customers = if customer_ids.is_empty() {
        HashMap::new()
    } else {
        CustomerEntity::find()
            .filter(customer::Column::Id.is_in(customer_ids))
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.name))
            .collect()
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows.drain(..) {
        let mut invoice_number = row.invoice_number;
        let mut customer_name = row.customer_name;
        let mut open_balance = row.open_balance;
        if let Some(inv) = model_by_id.get(&row.posted_invoice_id) {
            if invoice_number.is_empty() {
                invoice_number = posted_invoice_display_label(inv.id, &inv.number);
            }
            if customer_name.is_empty() {
                customer_name = customers
                    .get(&inv.customer_id)
                    .cloned()
                    .unwrap_or_else(|| "—".into());
            }
            if open_balance.is_empty() {
                if let Ok(open) = posted_invoice_open_balance(db, inv.id).await {
                    let currency = load_journal_currency_format(db, inv.journal_id).await;
                    open_balance = currency.display(open);
                }
            }
        }
        out.push(PaymentBatchAllocationRow {
            posted_invoice_id: row.posted_invoice_id,
            amount: row.amount,
            tax_ids: row.tax_ids,
            invoice_number,
            customer_name,
            open_balance,
        });
    }

    serde_json::to_string(&out).unwrap_or_else(|_| allocations_json.to_string())
}

async fn payment_batch_create_modal_page(
    state: &InvoicesState,
    q: &ModalNameQuery,
    form: PaymentBatchForm,
    error: String,
) -> PaymentBatchCreateModalPage {
    let account_id = form
        .account_id
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0);
    let account_display = load_account_parent_label(&state.db, account_id).await;
    let allocations_json = enrich_allocations_json(&state.db, &form.allocations_json).await;
    PaymentBatchCreateModalPage {
        form_name: q.form_name(),
        refresh_table: q.refresh_table(),
        form: PaymentBatchForm {
            allocations_json,
            ..form
        },
        account_display,
        batch_allocations_preview: batch_allocations_preview(&state.db).await,
        error,
    }
}

pub async fn create_get(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    Query(q): Query<BatchCreateQuery>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/?tab=posted").into_response();
    }

    let posted_ids = q
        .posted_invoice_ids
        .as_deref()
        .map(parse_posted_invoice_ids)
        .unwrap_or_default();

    let (allocations_json, error) = match build_allocations_json(&state.db, &posted_ids).await {
        Ok(json) => (json, String::new()),
        Err(e) if !posted_ids.is_empty() => ("[]".into(), e),
        Err(_) => ("[]".into(), String::new()),
    };

    let page = payment_batch_create_modal_page(
        &state,
        &q.modal,
        PaymentBatchForm {
            datetime: ctx.datetime_local_input(Utc::now()).into_string(),
            account_id: String::new(),
            allocations_json,
        },
        error,
    )
    .await;
    html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}

pub async fn create_post(
    Cap(state): Cap<InvoicesState>,
    Cap(chrome): Cap<SharedChromeFolder>,
    RequireAuth(ctx): RequireAuth,
    htmx: Htmx,
    Query(q): Query<BatchCreateQuery>,
    HtmlFormBody(form): HtmlFormBody<PaymentBatchForm>,
) -> Response {
    if !require_superuser(&ctx) {
        return Redirect::to("/finance-invoices/?tab=posted").into_response();
    }

    let allocations = match parse_batch_allocations_json(&form.allocations_json) {
        Ok(a) => a,
        Err(e) => {
            let page = payment_batch_create_modal_page(&state, &q.modal, form, e).await;
            return html_built_page_with_slots(&page, &chrome, &SlotCtx::from_auth(&ctx))
                .into_response();
        }
    };

    let account_id = form
        .account_id
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0);

    let input = CreatePaymentBatchInput {
        datetime: parse_invoice_datetime(&form.datetime, &ctx.timezone),
        account_id,
        allocations,
    };

    match create_payment_batch(&state.db, input).await {
        Ok(result) => respond_create_modal_done::<PaymentBatchCreateModalKey>(
            &htmx,
            &q.modal.refresh_table(),
            &PaymentBatchDetailRouteTag::new(result.batch.id).url(),
        ),
        Err(e) => {
            let page = payment_batch_create_modal_page(&state, &q.modal, form, e).await;
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
) -> Response {
    let batch = crate::web::opt_or_log(
        PaymentBatchEntity::find_by_id(id).one(&state.db).await,
        "find by id",
    );

    let page = if let Some(b) = batch {
        let account_label = load_account_parent_label(&state.db, Some(b.account_id)).await;
        let payments = PaymentEntity::find()
            .filter(payment::Column::PaymentBatchId.eq(b.id))
            .all(&state.db)
            .await
            .unwrap_or_default();

        let posted_ids: Vec<i64> = payments.iter().map(|p| p.posted_invoice_id).collect();
        let posted_labels = if posted_ids.is_empty() {
            HashMap::new()
        } else {
            PostedInvoiceEntity::find()
                .filter(posted_invoice::Column::Id.is_in(posted_ids))
                .all(&state.db)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|inv| (inv.id, posted_invoice_display_label(inv.id, &inv.number)))
                .collect()
        };

        let mut payment_rows = Vec::with_capacity(payments.len());
        for p in payments {
            let tax_ids = crate::plugins::finance_invoices::logic::tax_assoc::load_payment_tax_ids(
                &state.db, p.id,
            )
            .await
            .unwrap_or_default();
            let taxes = load_taxes_by_ids(&state.db, &tax_ids)
                .await
                .unwrap_or_default();
            let tax_labels = taxes.iter().map(tax_label).collect::<Vec<_>>().join(", ");

            let pay_currency =
                load_journal_entry_currency_format(&state.db, p.journal_entry_id).await;
            payment_rows.push(PaymentBatchPaymentRow {
                id: p.id,
                href: PaymentDetailRouteTag::new(p.id).url(),
                invoice_label: posted_labels
                    .get(&p.posted_invoice_id)
                    .cloned()
                    .unwrap_or_else(|| "—".into()),
                invoice_href: PostedInvoiceDetailRouteTag::new(p.posted_invoice_id).url(),
                amount: pay_currency.display(p.amount),
                tax_labels: if tax_labels.is_empty() {
                    "—".into()
                } else {
                    tax_labels
                },
            });
        }

        let batch_currency =
            load_journal_entry_currency_format(&state.db, b.journal_entry_id).await;
        PaymentBatchDetailPage {
            id: b.id,
            datetime: ctx.format_datetime_short(b.datetime).into_string(),
            account_label,
            total_amount: batch_currency.display(b.total_amount),
            journal_entry_id: b.journal_entry_id,
            payments: payment_rows,
            can_edit: require_superuser(&ctx),
        }
    } else {
        PaymentBatchDetailPage {
            id,
            datetime: String::new(),
            account_label: String::new(),
            total_amount: String::new(),
            journal_entry_id: 0,
            payments: vec![],
            can_edit: false,
        }
    };

    html_built_page_or_app_layout(&page, &htmx, &chrome, &SlotCtx::from_auth(&ctx)).into_response()
}
