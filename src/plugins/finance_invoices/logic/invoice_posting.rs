//! Draft → Posted, Posted → Cancelled, Cancelled → New draft (invoice_posting.go).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement,
    TransactionTrait,
};

use crate::plugins::finance_accounts::logic::journal::{
    JournalLineSpec, create_source_doc, insert_journal_entry, update_source_doc_id,
};
use crate::plugins::finance_accounts::scope::load_journal_entry_items;
use crate::plugins::finance_common::decimal;
use crate::plugins::finance_creditnotes::logic::{CreateCreditNoteInput, create_credit_note};
use crate::plugins::finance_products::preferences::{load_product_preferences, optional_i64};
use crate::plugins::finance_taxes::entities::tax::Model as TaxModel;
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;

use crate::plugins::finance_invoices::entities::{
    CancelledInvoiceEntity, DraftInvoiceEntity, DraftInvoiceLineEntity, PostedInvoiceEntity,
    PostedInvoiceLineEntity,
};
use crate::plugins::finance_invoices::entities::{
    cancelled_invoice, draft_invoice, draft_invoice_line, posted_invoice, posted_invoice_line,
};
use crate::plugins::finance_invoices::logic::draft_payment_term::{
    convert_draft_to_posted_payment_term, copy_posted_payment_term, posted_payment_term_to_draft,
};
use crate::plugins::finance_invoices::logic::invoice_number::posted_invoice_number;
use crate::plugins::finance_invoices::logic::preferences::{
    load_invoice_preferences, validate_invoice_preferences_for_posting,
};
use crate::plugins::finance_invoices::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_draft_invoice_tax_ids,
    load_draft_line_tax_ids, load_posted_invoice_tax_ids, load_posted_line_tax_ids,
    set_cancelled_invoice_taxes, set_cancelled_line_taxes, set_draft_invoice_taxes,
    set_draft_line_taxes, set_posted_invoice_taxes, set_posted_line_taxes,
};
use crate::plugins::finance_invoices::logic::tax_calculations::{
    InvoiceLinesTotals, document_level_header_taxes, invoice_line_amount_breakdown,
    invoice_receivable_grand_total, merge_invoice_line_tax_ids, tax_amount_for_tax,
    tax_amount_on_base, taxes_levied, taxes_withholding, validate_withholding_tax_accounts,
    withholding_tax_account_id,
};
use crate::plugins::finance_invoices::scope::find_cancellable_posted;

use crate::plugins::finance_invoices::entities::posted_invoice::POSTED_INVOICE_SOURCE_DOC_TYPE;

struct LineWithTaxes {
    line: draft_invoice_line::Model,
    taxes: Vec<TaxModel>,
    product_base_cost: Decimal,
}

pub async fn draft_new_posted(
    db: &DatabaseConnection,
    draft_id: i64,
    posted_at: DateTime<Utc>,
) -> Result<posted_invoice::Model, String> {
    let draft = DraftInvoiceEntity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("draft invoice required")?;

    let posted_count = PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::DraftInvoiceId.eq(draft_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if posted_count > 0 {
        return Err("draft already posted".to_string());
    }

    let lines = DraftInvoiceLineEntity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() {
        return Err("draft has no lines".to_string());
    }

    let header_tax_ids = load_draft_invoice_tax_ids(db, draft_id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    let mut all_taxes = header_taxes.clone();
    let mut lines_with_taxes = Vec::with_capacity(lines.len());
    for line in lines {
        let tax_ids = load_draft_line_tax_ids(db, line.id)
            .await
            .map_err(|e| e.to_string())?;
        let taxes = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        all_taxes.extend(taxes.clone());
        let product = crate::plugins::finance_products::entities::product::Entity::find_by_id(
            line.product_id,
        )
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("product not found")?;
        lines_with_taxes.push(LineWithTaxes {
            line,
            taxes,
            product_base_cost: product.base_cost,
        });
    }

    validate_withholding_tax_accounts(&all_taxes)?;

    let product_prefs = load_product_preferences(db).await;
    if optional_i64(product_prefs.inventory_account_id) == 0
        || optional_i64(product_prefs.cost_of_sales_account_id) == 0
    {
        return Err(
            "product preferences must have inventory and cost-of-sales accounts for posting"
                .to_string(),
        );
    }

    let invoice_prefs = load_invoice_preferences(db).await;
    validate_invoice_preferences_for_posting(db, &invoice_prefs).await?;

    let number = posted_invoice_number(db, &draft).await?;
    let dup_posted = PostedInvoiceEntity::find()
        .filter(posted_invoice::Column::Number.eq(&number))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if dup_posted > 0 {
        return Err(format!(
            "invoice number {number} is already used by another posted invoice"
        ));
    }

    let posted_at = if posted_at.timestamp() == 0 {
        Utc::now()
    } else {
        posted_at
    };

    let ar_id = optional_i64(invoice_prefs.account_receivable_id);
    let rev_id = optional_i64(invoice_prefs.account_revenue_id);
    let tax_pay_id = optional_i64(invoice_prefs.account_tax_payable_id);
    let journal_id = optional_i64(invoice_prefs.journal_id);
    let inv_id = optional_i64(product_prefs.inventory_account_id);
    let cogs_id = optional_i64(product_prefs.cost_of_sales_account_id);

    let mut specs: Vec<JournalLineSpec> = Vec::new();
    let mut rev_item_indices: Vec<usize> = Vec::new();

    for lwt in &lines_with_taxes {
        let line_base = decimal::dec_mul(lwt.line.quantity, lwt.line.rate);
        let levied_refs: Vec<_> = taxes_levied(&lwt.taxes);
        let levied_pct: Decimal = levied_refs.iter().map(|t| t.percentage).sum();
        let levied_tax = tax_amount_on_base(line_base, levied_pct);
        rev_item_indices.push(specs.len());
        specs.push(JournalLineSpec {
            account_id: rev_id,
            amount: decimal::dec_neg(line_base),
        });
        if !decimal::dec_is_zero(levied_tax) {
            specs.push(JournalLineSpec {
                account_id: tax_pay_id,
                amount: decimal::dec_neg(levied_tax),
            });
        }
        for tax in taxes_withholding(&lwt.taxes) {
            let wh = tax_amount_for_tax(line_base, tax);
            if decimal::dec_is_zero(wh) {
                continue;
            }
            specs.push(JournalLineSpec {
                account_id: withholding_tax_account_id(tax)?,
                amount: wh,
            });
        }
    }

    for lwt in &lines_with_taxes {
        let cost_base = decimal::dec_mul(lwt.product_base_cost, lwt.line.quantity);
        specs.push(JournalLineSpec {
            account_id: cogs_id,
            amount: cost_base,
        });
        specs.push(JournalLineSpec {
            account_id: inv_id,
            amount: decimal::dec_neg(cost_base),
        });
    }

    let mut line_totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for lwt in &lines_with_taxes {
        let (u, lev, wh, _) =
            invoice_line_amount_breakdown(lwt.line.quantity, lwt.line.rate, &lwt.taxes);
        line_totals.untaxed_subtotal = decimal::dec_sum(line_totals.untaxed_subtotal, u);
        line_totals.lines_levied = decimal::dec_sum(line_totals.lines_levied, lev);
        line_totals.lines_withholding = decimal::dec_sum(line_totals.lines_withholding, wh);
        merge_invoice_line_tax_ids(&mut line_tax_ids, &lwt.taxes);
    }

    for tax in document_level_header_taxes(&header_taxes, &line_tax_ids) {
        let amt = tax_amount_for_tax(line_totals.untaxed_subtotal, &tax);
        if decimal::dec_is_zero(amt) {
            continue;
        }
        if tax.tax_type == crate::plugins::finance_taxes::entities::TaxKind::Withholding {
            specs.push(JournalLineSpec {
                account_id: withholding_tax_account_id(&tax)?,
                amount: amt,
            });
        } else {
            specs.push(JournalLineSpec {
                account_id: tax_pay_id,
                amount: decimal::dec_neg(amt),
            });
        }
    }

    let total_ar = invoice_receivable_grand_total(&line_totals, &header_taxes, &line_tax_ids);
    specs.push(JournalLineSpec {
        account_id: ar_id,
        amount: total_ar,
    });

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let doc_id = create_source_doc(&txn, POSTED_INVOICE_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;
    let (je_id, je_items) = insert_journal_entry(&txn, posted_at, journal_id, doc_id, &specs)
        .await
        .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let payment_term =
        convert_draft_to_posted_payment_term(&txn, draft.id, draft.datetime, total_ar).await?;
    let posted_am = posted_invoice::ActiveModel {
        draft_invoice_id: Set(draft.id),
        posted_at: Set(Some(posted_at)),
        number: Set(number),
        reference: Set(draft.reference.clone()),
        payment_reference: Set(draft.payment_reference.clone()),
        bank_account: Set(draft.bank_account.clone()),
        account_receivable_id: Set(ar_id),
        account_revenue_id: Set(rev_id),
        account_tax_payable_id: Set(tax_pay_id),
        journal_id: Set(journal_id),
        datetime: Set(draft.datetime),
        customer_id: Set(draft.customer_id),
        journal_entry_id: Set(je_id),
        posted_payment_term_id: Set(Some(payment_term.id)),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let posted = posted_am.insert(&txn).await.map_err(|e| e.to_string())?;
    update_source_doc_id(&txn, doc_id, posted.id)
        .await
        .map_err(|e| e.to_string())?;

    let header_tax_ids_only: Vec<i64> = header_taxes.iter().map(|t| t.id).collect();
    set_posted_invoice_taxes(&txn, posted.id, &header_tax_ids_only)
        .await
        .map_err(|e| e.to_string())?;

    for (i, lwt) in lines_with_taxes.iter().enumerate() {
        let rev_idx = rev_item_indices[i];
        let rev_item_id = je_items
            .get(rev_idx)
            .map(|it| it.id)
            .ok_or("internal error: revenue item index")?;
        let pl_am = posted_invoice_line::ActiveModel {
            posted_invoice_id: Set(posted.id),
            product_id: Set(lwt.line.product_id),
            rate: Set(lwt.line.rate),
            quantity: Set(lwt.line.quantity),
            journal_entry_item_id: Set(rev_item_id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let pl = pl_am.insert(&txn).await.map_err(|e| e.to_string())?;
        let tax_ids: Vec<i64> = lwt.taxes.iter().map(|t| t.id).collect();
        set_posted_line_taxes(&txn, pl.id, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(posted)
}

pub async fn posted_new_cancelled(
    db: &DatabaseConnection,
    posted_id: i64,
    reason: String,
    at: DateTime<Utc>,
) -> Result<cancelled_invoice::Model, String> {
    let posted = find_cancellable_posted(db, posted_id)
        .await
        .ok_or("posted invoice is not cancellable")?;

    let posted_lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .order_by_asc(posted_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let header_tax_ids = load_posted_invoice_tax_ids(db, posted_id)
        .await
        .map_err(|e| e.to_string())?;

    let at = if at.timestamp() == 0 { Utc::now() } else { at };

    let cn = create_credit_note(
        db,
        CreateCreditNoteInput {
            datetime: at,
            reason,
            journal_entry_id: posted.journal_entry_id,
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    let orig_items: Vec<_> = load_journal_entry_items(db, posted.journal_entry_id)
        .await
        .into_iter()
        .map(|(item, _)| item)
        .collect();
    let rev_items: Vec<_> = load_journal_entry_items(db, cn.reversed_journal_entry_id)
        .await
        .into_iter()
        .map(|(item, _)| item)
        .collect();
    if orig_items.len() != rev_items.len() {
        return Err("reversal line count mismatch".to_string());
    }
    let orig_to_rev: HashMap<i64, i64> = orig_items
        .iter()
        .zip(rev_items.iter())
        .map(|(o, r)| (o.id, r.id))
        .collect();

    let now = Utc::now();
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let payment_term_id =
        copy_posted_payment_term(&txn, posted.posted_payment_term_id).await?;
    let cam = cancelled_invoice::ActiveModel {
        posted_invoice_id: Set(posted.id),
        posted_at: Set(posted.posted_at),
        cancelled_at: Set(Some(at)),
        number: Set(posted.number.clone()),
        reference: Set(posted.reference.clone()),
        payment_reference: Set(posted.payment_reference.clone()),
        bank_account: Set(posted.bank_account.clone()),
        account_receivable_id: Set(posted.account_receivable_id),
        account_revenue_id: Set(posted.account_revenue_id),
        account_tax_payable_id: Set(posted.account_tax_payable_id),
        journal_id: Set(posted.journal_id),
        datetime: Set(posted.datetime),
        customer_id: Set(posted.customer_id),
        credit_note_id: Set(cn.id),
        posted_payment_term_id: Set(payment_term_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let cancelled = cam.insert(&txn).await.map_err(|e| e.to_string())?;

    set_cancelled_invoice_taxes(&txn, cancelled.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for pl in posted_lines {
        let rev_id = orig_to_rev.get(&pl.journal_entry_item_id).ok_or_else(|| {
            format!(
                "could not map journal line for posted invoice line {}",
                pl.id
            )
        })?;
        let line_tax_ids = load_posted_line_tax_ids(&txn, pl.id)
            .await
            .map_err(|e| e.to_string())?;
        let cl_id = insert_cancelled_line(
            &txn,
            cancelled.id,
            pl.product_id,
            pl.rate,
            pl.quantity,
            *rev_id,
            now,
        )
        .await?;
        set_cancelled_line_taxes(&txn, cl_id, &line_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(cancelled)
}

pub async fn cancelled_new_draft(
    db: &DatabaseConnection,
    cancelled_id: i64,
    tz: &str,
) -> Result<draft_invoice::Model, String> {
    let cancelled = CancelledInvoiceEntity::find_by_id(cancelled_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("cancelled invoice required")?;

    let header_tax_ids = load_cancelled_invoice_tax_ids(db, cancelled_id)
        .await
        .map_err(|e| e.to_string())?;
    let cancelled_lines = load_cancelled_invoice_lines(db, cancelled_id).await?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let draft = draft_invoice::ActiveModel {
        number: Set(None),
        reference: Set(cancelled.reference.clone()),
        payment_reference: Set(cancelled.payment_reference.clone()),
        bank_account: Set(cancelled.bank_account.clone()),
        datetime: Set(cancelled.datetime),
        customer_id: Set(cancelled.customer_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    posted_payment_term_to_draft(&txn, cancelled.posted_payment_term_id, draft.id, tz).await?;

    set_draft_invoice_taxes(&txn, draft.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for cl in cancelled_lines {
        let line_tax_ids = load_cancelled_line_tax_ids(&txn, cl.id)
            .await
            .map_err(|e| e.to_string())?;
        let line = draft_invoice_line::ActiveModel {
            draft_invoice_id: Set(draft.id),
            product_id: Set(cl.product_id),
            rate: Set(cl.rate),
            quantity: Set(cl.quantity),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        set_draft_line_taxes(&txn, line.id, &line_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(draft)
}

struct CancelledLineSnapshot {
    id: i64,
    product_id: i64,
    rate: Decimal,
    quantity: Decimal,
}

async fn load_cancelled_invoice_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Result<Vec<CancelledLineSnapshot>, String> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, product_id, rate, quantity FROM cancelled_invoice_lines \
             WHERE cancelled_invoice_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|r| {
            Ok(CancelledLineSnapshot {
                id: r.try_get("", "id").map_err(|e| e.to_string())?,
                product_id: r.try_get("", "product_id").map_err(|e| e.to_string())?,
                rate: r.try_get("", "rate").map_err(|e| e.to_string())?,
                quantity: r.try_get("", "quantity").map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

async fn insert_cancelled_line<C: ConnectionTrait>(
    db: &C,
    cancelled_id: i64,
    product_id: i64,
    rate: Decimal,
    quantity: Decimal,
    journal_entry_item_id: i64,
    now: DateTime<Utc>,
) -> Result<i64, String> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO cancelled_invoice_lines \
             (cancelled_invoice_id, product_id, rate, quantity, journal_entry_item_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $6) RETURNING id",
            [
                cancelled_id.into(),
                product_id.into(),
                rate.into(),
                quantity.into(),
                journal_entry_item_id.into(),
                now.into(),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "insert cancelled line failed".to_string())?;
    row.try_get("", "id").map_err(|e| e.to_string())
}
