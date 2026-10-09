//! Draft → Posted, Posted → Cancelled, Cancelled → New draft (invoice_posting.go).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement,
    TransactionTrait,
};

use lariv_plugin_finance_accounts::logic::journal::{
    JournalLineSpec, create_source_doc, insert_journal_entry, update_source_doc_id,
};
use lariv_plugin_finance_accounts::scope::load_journal_entry_items;
use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_creditnotes::logic::{CreateCreditNoteInput, create_credit_note};
use lariv_plugin_finance_products::preferences::{load_product_preferences, optional_i64};
use lariv_plugin_finance_products::pricing;
use lariv_plugin_finance_taxes::entities::tax::Model as TaxModel;
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::entities::{
    CancelledInvoiceEntity, DraftInvoiceEntity, DraftInvoiceLineEntity, PostedInvoiceEntity,
    PostedInvoiceLineEntity,
};
use crate::entities::{
    cancelled_invoice, draft_invoice, draft_invoice_line, posted_invoice, posted_invoice_line,
};
use crate::logic::draft_payment_term::{
    convert_draft_to_posted_payment_term, copy_posted_payment_term, posted_payment_term_to_draft,
};
use crate::logic::invoice_number::posted_invoice_number;
use crate::logic::preferences::{
    load_invoice_preferences, validate_invoice_preferences_for_posting,
};
use crate::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_draft_invoice_tax_ids,
    load_draft_line_tax_ids, load_posted_invoice_tax_ids, load_posted_line_tax_ids,
    set_cancelled_invoice_taxes, set_cancelled_line_taxes, set_draft_invoice_taxes,
    set_draft_line_taxes, set_posted_invoice_taxes, set_posted_line_taxes,
};
use crate::logic::tax_calculations::{
    InvoiceLinesTotals, document_level_header_taxes, invoice_line_amounts,
    invoice_receivable_grand_total, merge_invoice_line_tax_ids, tax_amount_for_tax,
    tax_amount_on_base, taxes_levied, taxes_withholding, validate_withholding_tax_accounts,
    withholding_tax_account_id,
};
use crate::scope::find_cancellable_posted;

use crate::entities::posted_invoice::POSTED_INVOICE_SOURCE_DOC_TYPE;

struct LineWithTaxes {
    line: draft_invoice_line::Model,
    taxes: Vec<TaxModel>,
    cost_amount: Decimal,
}

/// Accounts used to assemble an invoice's journal lines.
pub struct InvoiceGlAccounts {
    pub receivable_id: i64,
    pub revenue_id: i64,
    pub tax_payable_id: i64,
    pub inventory_id: i64,
    pub cost_of_sales_id: i64,
}

/// One invoice line's amounts, as stored on the draft or posted document.
pub struct InvoiceJournalLineInput {
    pub pre_tax_amount: Decimal,
    pub taxes: Vec<TaxModel>,
    pub cost_amount: Decimal,
}

/// Balanced invoice lines plus the revenue-item index for each input line.
pub struct AssembledInvoiceJournal {
    pub lines: Vec<JournalLineSpec>,
    pub revenue_item_indices: Vec<usize>,
    pub receivable_total: Decimal,
}

/// Build receivable, revenue, tax, inventory, and cost-of-sales lines.
pub fn assemble_invoice_journal_lines(
    lines: &[InvoiceJournalLineInput],
    header_taxes: &[TaxModel],
    accounts: InvoiceGlAccounts,
) -> Result<AssembledInvoiceJournal, String> {
    let mut specs: Vec<JournalLineSpec> = Vec::new();
    let mut revenue_item_indices: Vec<usize> = Vec::new();

    for line in lines {
        let line_base = line.pre_tax_amount;
        let levied_refs: Vec<_> = taxes_levied(&line.taxes);
        let levied_pct: Decimal = levied_refs.iter().map(|t| t.percentage).sum();
        let levied_tax = tax_amount_on_base(line_base, levied_pct);
        revenue_item_indices.push(specs.len());
        specs.push(JournalLineSpec {
            account_id: accounts.revenue_id,
            amount: decimal::dec_neg(line_base),
        });
        if !decimal::dec_is_zero(levied_tax) {
            specs.push(JournalLineSpec {
                account_id: accounts.tax_payable_id,
                amount: decimal::dec_neg(levied_tax),
            });
        }
        for tax in taxes_withholding(&line.taxes) {
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

    for line in lines {
        specs.push(JournalLineSpec {
            account_id: accounts.cost_of_sales_id,
            amount: line.cost_amount,
        });
        specs.push(JournalLineSpec {
            account_id: accounts.inventory_id,
            amount: decimal::dec_neg(line.cost_amount),
        });
    }

    let mut line_totals = InvoiceLinesTotals::default();
    let mut line_tax_ids = HashSet::new();
    for line in lines {
        let (u, lev, wh, _) = invoice_line_amounts(line.pre_tax_amount, &line.taxes);
        line_totals.untaxed_subtotal = decimal::dec_sum(line_totals.untaxed_subtotal, u);
        line_totals.lines_levied = decimal::dec_sum(line_totals.lines_levied, lev);
        line_totals.lines_withholding = decimal::dec_sum(line_totals.lines_withholding, wh);
        merge_invoice_line_tax_ids(&mut line_tax_ids, &line.taxes);
    }

    for tax in document_level_header_taxes(header_taxes, &line_tax_ids) {
        let amt = tax_amount_for_tax(line_totals.untaxed_subtotal, &tax);
        if decimal::dec_is_zero(amt) {
            continue;
        }
        if tax.tax_type == lariv_plugin_finance_taxes::entities::TaxKind::Withholding {
            specs.push(JournalLineSpec {
                account_id: withholding_tax_account_id(&tax)?,
                amount: amt,
            });
        } else {
            specs.push(JournalLineSpec {
                account_id: accounts.tax_payable_id,
                amount: decimal::dec_neg(amt),
            });
        }
    }

    let receivable_total =
        invoice_receivable_grand_total(&line_totals, header_taxes, &line_tax_ids);
    specs.push(JournalLineSpec {
        account_id: accounts.receivable_id,
        amount: receivable_total,
    });

    Ok(AssembledInvoiceJournal {
        lines: specs,
        revenue_item_indices,
        receivable_total,
    })
}

/// Recompute a posted invoice's journal lines from its stored accounts, lines, and taxes.
pub async fn posted_invoice_journal_lines(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<(DateTime<Utc>, Vec<JournalLineSpec>, Vec<usize>), String> {
    let posted = PostedInvoiceEntity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("posted invoice {posted_id} not found"))?;

    let lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .order_by_asc(posted_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() {
        return Err(format!("posted invoice {posted_id} has no lines"));
    }

    let header_tax_ids = load_posted_invoice_tax_ids(db, posted_id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    let product_prefs = load_product_preferences(db).await;
    let inventory_id = optional_i64(product_prefs.inventory_account_id);
    let cost_of_sales_id = optional_i64(product_prefs.cost_of_sales_account_id);
    if inventory_id == 0 || cost_of_sales_id == 0 {
        return Err(
            "product preferences must have inventory and cost-of-sales accounts for posting"
                .to_string(),
        );
    }

    let mut inputs = Vec::with_capacity(lines.len());
    for line in lines {
        let tax_ids = load_posted_line_tax_ids(db, line.id)
            .await
            .map_err(|e| e.to_string())?;
        let taxes = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        let product =
            lariv_plugin_finance_products::entities::product::Entity::find_by_id(line.product_id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("product {} not found", line.product_id))?;
        let cost_amount = pricing::line_cost(
            &product.variables,
            &product.base_price_formula,
            &line.variable_values,
            line.quantity,
        )?;
        inputs.push(InvoiceJournalLineInput {
            pre_tax_amount: line.pre_tax_amount,
            taxes,
            cost_amount,
        });
    }

    let assembled = assemble_invoice_journal_lines(
        &inputs,
        &header_taxes,
        InvoiceGlAccounts {
            receivable_id: posted.account_receivable_id,
            revenue_id: posted.account_revenue_id,
            tax_payable_id: posted.account_tax_payable_id,
            inventory_id,
            cost_of_sales_id,
        },
    )?;
    Ok((
        posted.datetime,
        assembled.lines,
        assembled.revenue_item_indices,
    ))
}

/// Point a posted invoice, its lines, and credit notes that reverse it at a new entry.
pub async fn adopt_posted_invoice_journal_entry<C: ConnectionTrait>(
    db: &C,
    posted_id: i64,
    journal_id: i64,
    journal_entry_id: i64,
    item_ids: &[i64],
    line_item_indexes: &[usize],
) -> Result<(), String> {
    let posted = PostedInvoiceEntity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("posted invoice {posted_id} not found"))?;
    let old_entry_id = posted.journal_entry_id;
    let lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .order_by_asc(posted_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.len() != line_item_indexes.len() {
        return Err(format!(
            "posted invoice {posted_id} line count {} does not match {} journal item indexes",
            lines.len(),
            line_item_indexes.len()
        ));
    }
    for (line, index) in lines.into_iter().zip(line_item_indexes.iter().copied()) {
        let item_id = *item_ids.get(index).ok_or_else(|| {
            format!("posted invoice {posted_id} journal item index {index} is out of range")
        })?;
        let mut am: posted_invoice_line::ActiveModel = line.into();
        am.journal_entry_item_id = Set(item_id);
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await.map_err(|e| e.to_string())?;
    }

    let mut am: posted_invoice::ActiveModel = posted.into();
    am.journal_entry_id = Set(journal_entry_id);
    am.journal_id = Set(journal_id);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(|e| e.to_string())?;

    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE credit_notes SET journal_entry_id = $1, updated_at = NOW() WHERE journal_entry_id = $2",
        [journal_entry_id.into(), old_entry_id.into()],
    ))
    .await
    .map_err(|e| e.to_string())?;
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE cancelled_invoices SET journal_id = $1, updated_at = NOW() WHERE posted_invoice_id = $2",
        [journal_id.into(), posted_id.into()],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn draft_new_posted(
    db: &DatabaseConnection,
    draft_id: i64,
    posted_at: DateTime<Utc>,
    tz: &str,
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
        let product =
            lariv_plugin_finance_products::entities::product::Entity::find_by_id(line.product_id)
                .one(db)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("product not found")?;
        let cost_amount = pricing::line_cost(
            &product.variables,
            &product.base_price_formula,
            &line.variable_values,
            line.quantity,
        )?;
        lines_with_taxes.push(LineWithTaxes {
            line,
            taxes,
            cost_amount,
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
    let source_doc_datetime = if draft.datetime.timestamp() == 0 {
        posted_at
    } else {
        draft.datetime
    };

    let ar_id = optional_i64(invoice_prefs.account_receivable_id);
    let rev_id = optional_i64(invoice_prefs.account_revenue_id);
    let tax_pay_id = optional_i64(invoice_prefs.account_tax_payable_id);
    let journal_id = optional_i64(invoice_prefs.journal_id);
    let inv_id = optional_i64(product_prefs.inventory_account_id);
    let cogs_id = optional_i64(product_prefs.cost_of_sales_account_id);

    let inputs: Vec<InvoiceJournalLineInput> = lines_with_taxes
        .iter()
        .map(|lwt| InvoiceJournalLineInput {
            pre_tax_amount: lwt.line.pre_tax_amount,
            taxes: lwt.taxes.clone(),
            cost_amount: lwt.cost_amount,
        })
        .collect();
    let assembled = assemble_invoice_journal_lines(
        &inputs,
        &header_taxes,
        InvoiceGlAccounts {
            receivable_id: ar_id,
            revenue_id: rev_id,
            tax_payable_id: tax_pay_id,
            inventory_id: inv_id,
            cost_of_sales_id: cogs_id,
        },
    )?;
    let specs = assembled.lines;
    let rev_item_indices = assembled.revenue_item_indices;
    let total_ar = assembled.receivable_total;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let doc_id = create_source_doc(&txn, POSTED_INVOICE_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;
    let (je_id, je_items) =
        insert_journal_entry(&txn, source_doc_datetime, journal_id, doc_id, &specs)
            .await
            .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let payment_term =
        convert_draft_to_posted_payment_term(&txn, draft.id, draft.datetime, total_ar, tz).await?;
    let posted_am = posted_invoice::ActiveModel {
        draft_invoice_id: Set(draft.id),
        posted_at: Set(Some(posted_at)),
        number: Set(number),
        reference: Set(draft.reference.clone()),
        payment_reference: Set(draft.payment_reference.clone()),
        bank_account: Set(draft.bank_account.clone()),
        remarks: Set(draft.remarks.clone()),
        account_receivable_id: Set(ar_id),
        account_revenue_id: Set(rev_id),
        account_tax_payable_id: Set(tax_pay_id),
        journal_id: Set(journal_id),
        datetime: Set(draft.datetime),
        delivery_date: Set(draft.delivery_date),
        bill_to_individual: Set(draft.bill_to_individual),
        customer_individual: Set(draft.customer_individual),
        customer_company: Set(draft.customer_company),
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
            variable_values: Set(lwt.line.variable_values.clone()),
            pre_tax_amount: Set(lwt.line.pre_tax_amount),
            remarks: Set(lwt.line.remarks.clone()),
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

    let mut orig_items: Vec<_> = load_journal_entry_items(db, posted.journal_entry_id)
        .await
        .into_iter()
        .map(|(item, _)| item)
        .collect();
    let mut rev_items: Vec<_> = load_journal_entry_items(db, cn.reversed_journal_entry_id)
        .await
        .into_iter()
        .map(|(item, _)| item)
        .collect();
    orig_items.sort_by_key(|item| item.id);
    rev_items.sort_by_key(|item| item.id);
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
    let payment_term_id = copy_posted_payment_term(&txn, posted.posted_payment_term_id).await?;
    let cam = cancelled_invoice::ActiveModel {
        posted_invoice_id: Set(posted.id),
        posted_at: Set(posted.posted_at),
        cancelled_at: Set(Some(at)),
        number: Set(posted.number.clone()),
        reference: Set(posted.reference.clone()),
        payment_reference: Set(posted.payment_reference.clone()),
        bank_account: Set(posted.bank_account.clone()),
        remarks: Set(posted.remarks.clone()),
        account_receivable_id: Set(posted.account_receivable_id),
        account_revenue_id: Set(posted.account_revenue_id),
        account_tax_payable_id: Set(posted.account_tax_payable_id),
        journal_id: Set(posted.journal_id),
        datetime: Set(posted.datetime),
        delivery_date: Set(posted.delivery_date),
        bill_to_individual: Set(posted.bill_to_individual),
        customer_individual: Set(posted.customer_individual),
        customer_company: Set(posted.customer_company),
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
            &pl.variable_values,
            pl.pre_tax_amount,
            pl.remarks.clone(),
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
    _tz: &str,
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
        remarks: Set(cancelled.remarks.clone()),
        datetime: Set(cancelled.datetime),
        delivery_date: Set(cancelled.delivery_date),
        bill_to_individual: Set(cancelled.bill_to_individual),
        customer_individual: Set(cancelled.customer_individual),
        customer_company: Set(cancelled.customer_company),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    posted_payment_term_to_draft(&txn, cancelled.posted_payment_term_id, draft.id).await?;

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
            variable_values: Set(cl.variable_values.clone()),
            pre_tax_amount: Set(cl.pre_tax_amount),
            remarks: Set(cl.remarks.clone()),
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
    variable_values: String,
    pre_tax_amount: Decimal,
    remarks: Option<String>,
}

async fn load_cancelled_invoice_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Result<Vec<CancelledLineSnapshot>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, product_id, rate, quantity, variable_values, pre_tax_amount, remarks \
             FROM cancelled_invoice_lines \
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
                variable_values: r
                    .try_get("", "variable_values")
                    .map_err(|e| e.to_string())?,
                pre_tax_amount: r.try_get("", "pre_tax_amount").map_err(|e| e.to_string())?,
                remarks: r.try_get("", "remarks").map_err(|e| e.to_string())?,
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
    variable_values: &str,
    pre_tax_amount: Decimal,
    remarks: Option<String>,
    journal_entry_item_id: i64,
    now: DateTime<Utc>,
) -> Result<i64, String> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO cancelled_invoice_lines \
             (cancelled_invoice_id, product_id, rate, quantity, variable_values, pre_tax_amount, remarks, journal_entry_item_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9) RETURNING id",
            [
                cancelled_id.into(),
                product_id.into(),
                rate.into(),
                quantity.into(),
                variable_values.into(),
                pre_tax_amount.into(),
                remarks.into(),
                journal_entry_item_id.into(),
                now.into(),
            ],
        ))
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "insert cancelled line failed".to_string())?;
    row.try_get("", "id").map_err(|e| e.to_string())
}
