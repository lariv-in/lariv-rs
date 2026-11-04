//! Draft → Posted, Posted → Cancelled, Posted → New draft, Cancelled → New draft.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::sea_query::LockType;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Statement, TransactionTrait,
};

use lariv_plugin_finance_accounts::logic::custom_source_doc::{
    SubmittedJournalLines, doc_line_indexes, index_of_item, journal_specs_editor_json,
    stored_journal_line_specs,
};
use lariv_plugin_finance_accounts::logic::journal::{
    JournalLineSpec, create_source_doc, insert_journal_entry, update_source_doc_id,
};
use lariv_plugin_finance_accounts::scope::load_journal_entry_items;
use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_creditnotes::logic::{CreateCreditNoteInput, create_credit_note};
use lariv_plugin_finance_products::journal_template::{
    self, ParsedTemplateLine, evaluate_template_lines,
};
use lariv_plugin_finance_taxes::entities::tax::Model as TaxModel;
use lariv_plugin_finance_taxes::journal_template::evaluate_tax_for_document;
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::draft_form_addon::run_draft_invoice_on_posted;
use crate::entities::{
    CancelledInvoiceEntity, DraftInvoiceEntity, DraftInvoiceLineEntity, PostedInvoiceEntity,
    PostedInvoiceLineEntity,
};
use crate::entities::{
    cancelled_invoice, draft_invoice, draft_invoice_line, posted_invoice, posted_invoice_line,
};
use crate::logic::draft_payment_term::{
    convert_draft_to_posted_payment_term, copy_posted_payment_term,
};
use crate::logic::invoice_number::posted_invoice_number;
use crate::logic::tax_assoc::{
    load_cancelled_invoice_tax_ids, load_cancelled_line_tax_ids, load_draft_invoice_tax_ids,
    load_draft_line_tax_ids, load_posted_invoice_tax_ids, load_posted_line_tax_ids,
    set_cancelled_invoice_taxes, set_cancelled_line_taxes, set_posted_invoice_taxes,
    set_posted_line_taxes, set_quotation_invoice_taxes, set_quotation_line_taxes,
};
use crate::logic::tax_calculations::{
    InvoiceLinesTotals, document_level_header_taxes, invoice_line_amounts,
    invoice_receivable_grand_total, merge_invoice_line_tax_ids,
};
use crate::scope::find_cancellable_posted;

use crate::entities::posted_invoice::POSTED_INVOICE_SOURCE_DOC_TYPE;

struct LineWithTaxes {
    line: draft_invoice_line::Model,
    taxes: Vec<TaxModel>,
}

/// Journal lines for one document line, expanded from its product or service template.
struct ExpandedLineJournal {
    specs: Vec<JournalLineSpec>,
    journal_id: i64,
}

async fn expand_product_journal(
    db: &DatabaseConnection,
    product_id: Option<i64>,
    product_name: &str,
    variable_types: &str,
    sales_price_formula: &str,
    variable_values: &str,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
) -> Result<ExpandedLineJournal, String> {
    let product_id = product_id.ok_or_else(|| {
        format!(
            "{product_name} was deleted; restore the product or replace the line before posting"
        )
    })?;
    let Some(template) = journal_template::load_template(db, product_id).await? else {
        return Err(format!("{product_name} has no journal entry template"));
    };
    if template.items.is_empty() {
        return Err(format!(
            "{product_name} has no journal entry template lines"
        ));
    }
    let parsed: Vec<ParsedTemplateLine> = template
        .items
        .iter()
        .map(|item| ParsedTemplateLine {
            account_id: item.account_id,
            formula: item.formula.clone(),
        })
        .collect();
    let sales_price = lariv_plugin_finance_products::pricing::unit_formula_amount(
        variable_types,
        sales_price_formula,
        variable_values,
    )
    .map_err(|e| format!("{product_name} sales price: {e}"))?;
    let evaluated = evaluate_template_lines(
        variable_types,
        &parsed,
        variable_values,
        quantity,
        rate,
        pre_tax,
        sales_price,
    )
    .map_err(|e| format!("{product_name}: {e}"))?;
    Ok(ExpandedLineJournal {
        specs: evaluated
            .into_iter()
            .map(|line| JournalLineSpec {
                account_id: line.account_id,
                amount: line.amount,
            })
            .collect(),
        journal_id: template.journal_id,
    })
}

/// Append every tax template line onto the invoice journal.
async fn append_invoice_tax_lines(
    db: &DatabaseConnection,
    specs: &mut Vec<JournalLineSpec>,
    lines: &[(Decimal, Vec<TaxModel>)],
    header_taxes: &[TaxModel],
    journal_id: i64,
) -> Result<(), String> {
    let mut line_tax_ids = HashSet::new();
    let mut pre_tax_sum = Decimal::ZERO;
    for (base, taxes) in lines {
        pre_tax_sum = decimal::dec_sum(pre_tax_sum, *base);
        for tax in taxes {
            if tax.id != 0 {
                line_tax_ids.insert(tax.id);
            }
            let evaluated = evaluate_tax_for_document(db, tax, *base, journal_id).await?;
            push_tax_lines(specs, &evaluated);
        }
    }
    for tax in document_level_header_taxes(header_taxes, &line_tax_ids) {
        let evaluated = evaluate_tax_for_document(db, &tax, pre_tax_sum, journal_id).await?;
        push_tax_lines(specs, &evaluated);
    }
    Ok(())
}

fn push_tax_lines(
    specs: &mut Vec<JournalLineSpec>,
    evaluated: &[lariv_plugin_finance_taxes::journal_template::EvaluatedTemplateLine],
) {
    for line in evaluated {
        specs.push(JournalLineSpec {
            account_id: line.account_id,
            amount: line.amount,
        });
    }
}

/// Journal lines confirmed when the invoice was posted.
///
/// Regeneration copies these stored items. Template edits apply to the next post.
pub async fn posted_invoice_journal_lines(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<(DateTime<Utc>, Vec<JournalLineSpec>, Vec<usize>), String> {
    let posted = PostedInvoiceEntity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("posted invoice {posted_id} not found"))?;

    let stored = stored_journal_line_specs(db, posted.journal_entry_id).await?;
    let item_ids: Vec<i64> = stored.iter().map(|(id, _)| *id).collect();
    let specs = stored.into_iter().map(|(_, spec)| spec).collect();

    let lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .order_by_asc(posted_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() {
        return Err(format!("posted invoice {posted_id} has no lines"));
    }
    let mut indexes = Vec::with_capacity(lines.len());
    for line in &lines {
        indexes.push(index_of_item(&item_ids, line.journal_entry_item_id)?);
    }
    Ok((posted.datetime, specs, indexes))
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

/// Prefill for the post modal: template lines, with an anchor on each document line.
pub struct DraftJournalPreview {
    pub label: String,
    pub lines_json: String,
}

pub async fn preview_draft_journal(
    db: &DatabaseConnection,
    draft_id: i64,
    slot_prefix: &str,
) -> Result<DraftJournalPreview, String> {
    let prepared = prepare_draft_journal(db, draft_id).await?;
    let lines_json = journal_specs_editor_json(
        db,
        &prepared.specs,
        &prepared.line_item_indexes,
        slot_prefix,
    )
    .await?;
    Ok(DraftJournalPreview {
        label: draft_journal_label(prepared.draft.id, &prepared.draft.number),
        lines_json,
    })
}

fn draft_journal_label(id: i64, number: &Option<String>) -> String {
    match number.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(number) => format!("Draft invoice {number}"),
        None => format!("Draft invoice #{id}"),
    }
}

struct PreparedDraftJournal {
    draft: draft_invoice::Model,
    journal_id: i64,
    specs: Vec<JournalLineSpec>,
    line_item_indexes: Vec<usize>,
    lines_with_taxes: Vec<LineWithTaxes>,
    header_taxes: Vec<TaxModel>,
    total_due: Decimal,
}

async fn prepare_draft_journal(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<PreparedDraftJournal, String> {
    let draft = DraftInvoiceEntity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("draft invoice required")?;

    let lines = DraftInvoiceLineEntity::find()
        .filter(draft_invoice_line::Column::DraftInvoiceId.eq(draft_id))
        .order_by_asc(draft_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() {
        return Err("quotation has no lines".to_string());
    }

    let header_tax_ids = load_draft_invoice_tax_ids(db, draft_id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    let mut lines_with_taxes = Vec::with_capacity(lines.len());
    let mut specs = Vec::new();
    let mut line_item_indexes = Vec::with_capacity(lines.len());
    let mut journal_id = None;
    for line in lines {
        let tax_ids = load_draft_line_tax_ids(db, line.id)
            .await
            .map_err(|e| e.to_string())?;
        let taxes = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        let expanded = expand_product_journal(
            db,
            Some(line.product_id),
            &line.product_name,
            &line.variable_types,
            &line.sales_price_formula,
            &line.variable_values,
            line.quantity,
            line.rate,
            line.pre_tax_amount,
        )
        .await?;
        if let Some(existing) = journal_id {
            if existing != expanded.journal_id {
                return Err(
                    "every product and service on an invoice must use the same journal".to_string(),
                );
            }
        } else {
            journal_id = Some(expanded.journal_id);
        }
        line_item_indexes.push(specs.len());
        specs.extend(expanded.specs);
        lines_with_taxes.push(LineWithTaxes { line, taxes });
    }
    let journal_id = journal_id.ok_or("quotation has no lines")?;
    let tax_inputs: Vec<(Decimal, Vec<TaxModel>)> = lines_with_taxes
        .iter()
        .map(|lwt| (lwt.line.pre_tax_amount, lwt.taxes.clone()))
        .collect();
    append_invoice_tax_lines(db, &mut specs, &tax_inputs, &header_taxes, journal_id).await?;
    let mut totals = InvoiceLinesTotals::default();
    let mut all_line_tax_ids = HashSet::new();
    for (pre_tax_amount, taxes) in lines_with_taxes
        .iter()
        .map(|lwt| (lwt.line.pre_tax_amount, &lwt.taxes))
    {
        merge_invoice_line_tax_ids(&mut all_line_tax_ids, taxes);
        let (untaxed, levied, withholding, _) = invoice_line_amounts(pre_tax_amount, taxes);
        totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
        totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
        totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
    }
    let total_due = invoice_receivable_grand_total(&totals, &header_taxes, &all_line_tax_ids);
    Ok(PreparedDraftJournal {
        draft,
        journal_id,
        specs,
        line_item_indexes,
        lines_with_taxes,
        header_taxes,
        total_due,
    })
}

pub async fn draft_new_posted(
    db: &DatabaseConnection,
    draft_id: i64,
    posted_at: DateTime<Utc>,
    tz: &str,
) -> Result<posted_invoice::Model, String> {
    draft_new_posted_with_lines(db, draft_id, posted_at, tz, None).await
}

pub async fn draft_new_posted_with_lines(
    db: &DatabaseConnection,
    draft_id: i64,
    posted_at: DateTime<Utc>,
    tz: &str,
    submitted: Option<SubmittedJournalLines>,
) -> Result<posted_invoice::Model, String> {
    let PreparedDraftJournal {
        draft,
        journal_id,
        mut specs,
        mut line_item_indexes,
        lines_with_taxes,
        header_taxes,
        total_due,
    } = prepare_draft_journal(db, draft_id).await?;
    if let Some(submitted) = submitted {
        line_item_indexes = doc_line_indexes(line_item_indexes.len(), &submitted.anchors);
        specs = submitted.lines;
    }

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

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    // Serialize concurrent posts of the same draft (draft is deleted after success).
    let locked = DraftInvoiceEntity::find_by_id(draft.id)
        .lock(LockType::Update)
        .one(&txn)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("draft invoice required")?;
    let draft = locked;

    let doc_id = create_source_doc(&txn, POSTED_INVOICE_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;
    let (je_id, je_items) =
        insert_journal_entry(&txn, source_doc_datetime, journal_id, doc_id, &specs)
            .await
            .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let payment_term =
        convert_draft_to_posted_payment_term(&txn, draft.id, draft.datetime, total_due, tz).await?;
    let posted_am = posted_invoice::ActiveModel {
        posted_at: Set(Some(posted_at)),
        number: Set(number),
        reference: Set(draft.reference.clone()),
        eway_bill_number: Set(draft.eway_bill_number.clone()),
        payment_reference: Set(draft.payment_reference.clone()),
        bank_account: Set(draft.bank_account.clone()),
        remarks: Set(draft.remarks.clone()),
        journal_id: Set(journal_id),
        datetime: Set(draft.datetime),
        delivery_date: Set(draft.delivery_date),
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
        let item_idx = line_item_indexes[i];
        let rev_item_id = je_items
            .get(item_idx)
            .map(|it| it.id)
            .ok_or("internal error: journal item index")?;
        let pl_am = posted_invoice_line::ActiveModel {
            posted_invoice_id: Set(posted.id),
            product_id: Set(Some(lwt.line.product_id)),
            product_name: Set(lwt.line.product_name.clone()),
            product_remarks: Set(lwt.line.product_remarks.clone()),
            hsn_code: Set(lwt.line.hsn_code),
            sac_code: Set(lwt.line.sac_code),
            variable_types: Set(lwt.line.variable_types.clone()),
            sales_price_formula: Set(lwt.line.sales_price_formula.clone()),
            rate: Set(lwt.line.rate),
            quantity: Set(lwt.line.quantity),
            qty_kind: Set(lwt.line.qty_kind.clone()),
            qty_unit: Set(lwt.line.qty_unit.clone()),
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

    run_draft_invoice_on_posted(&txn, draft.id, posted.id).await?;

    let term_id = draft.draft_payment_term_id;
    DraftInvoiceEntity::delete_by_id(draft.id)
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    // Trigger also deletes the term; keep this for DBs that have not applied m00021.
    if let Some(term_id) = term_id {
        use crate::entities::DraftPaymentTermEntity;
        let _ = DraftPaymentTermEntity::delete_by_id(term_id)
            .exec(&txn)
            .await;
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
        eway_bill_number: Set(posted.eway_bill_number.clone()),
        payment_reference: Set(posted.payment_reference.clone()),
        bank_account: Set(posted.bank_account.clone()),
        remarks: Set(posted.remarks.clone()),
        journal_id: Set(posted.journal_id),
        datetime: Set(posted.datetime),
        delivery_date: Set(posted.delivery_date),
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
            &pl.product_name,
            pl.product_remarks.as_deref(),
            pl.hsn_code,
            pl.sac_code,
            &pl.variable_types,
            &pl.sales_price_formula,
            pl.rate,
            pl.quantity,
            &pl.qty_kind,
            &pl.qty_unit,
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

pub async fn posted_new_draft(
    db: &DatabaseConnection,
    posted_id: i64,
    _tz: &str,
) -> Result<crate::entities::quotation_invoice::Model, String> {
    let posted = PostedInvoiceEntity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("posted invoice required")?;

    let header_tax_ids = load_posted_invoice_tax_ids(db, posted_id)
        .await
        .map_err(|e| e.to_string())?;
    let posted_lines = PostedInvoiceLineEntity::find()
        .filter(posted_invoice_line::Column::PostedInvoiceId.eq(posted_id))
        .order_by_asc(posted_invoice_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let quotation = crate::entities::quotation_invoice::ActiveModel {
        number: Set(None),
        reference: Set(posted.reference.clone()),
        eway_bill_number: Set(posted.eway_bill_number.clone()),
        payment_reference: Set(posted.payment_reference.clone()),
        bank_account: Set(posted.bank_account.clone()),
        remarks: Set(posted.remarks.clone()),
        datetime: Set(posted.datetime),
        delivery_date: Set(posted.delivery_date),
        customer_id: Set(posted.customer_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    crate::logic::quotation::copy_posted_payment_term_to_quotation(
        &txn,
        posted.posted_payment_term_id,
        quotation.id,
    )
    .await?;

    set_quotation_invoice_taxes(&txn, quotation.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for pl in posted_lines {
        let line_tax_ids = load_posted_line_tax_ids(&txn, pl.id)
            .await
            .map_err(|e| e.to_string())?;
        let line = crate::entities::quotation_invoice_line::ActiveModel {
            quotation_invoice_id: Set(quotation.id),
            product_id: Set(pl.product_id.unwrap_or(0)),
            product_name: Set(pl.product_name.clone()),
            product_remarks: Set(pl.product_remarks.clone()),
            hsn_code: Set(pl.hsn_code),
            sac_code: Set(pl.sac_code),
            variable_types: Set(pl.variable_types.clone()),
            sales_price_formula: Set(pl.sales_price_formula.clone()),
            rate: Set(pl.rate),
            quantity: Set(pl.quantity),
            qty_kind: Set(pl.qty_kind.clone()),
            qty_unit: Set(pl.qty_unit.clone()),
            variable_values: Set(pl.variable_values.clone()),
            pre_tax_amount: Set(pl.pre_tax_amount),
            remarks: Set(pl.remarks.clone()),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        set_quotation_line_taxes(&txn, line.id, &line_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(quotation)
}

pub async fn cancelled_new_draft(
    db: &DatabaseConnection,
    cancelled_id: i64,
    _tz: &str,
) -> Result<crate::entities::quotation_invoice::Model, String> {
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
    let quotation = crate::entities::quotation_invoice::ActiveModel {
        number: Set(None),
        reference: Set(cancelled.reference.clone()),
        eway_bill_number: Set(cancelled.eway_bill_number.clone()),
        payment_reference: Set(cancelled.payment_reference.clone()),
        bank_account: Set(cancelled.bank_account.clone()),
        remarks: Set(cancelled.remarks.clone()),
        datetime: Set(cancelled.datetime),
        delivery_date: Set(cancelled.delivery_date),
        customer_id: Set(cancelled.customer_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    crate::logic::quotation::copy_posted_payment_term_to_quotation(
        &txn,
        cancelled.posted_payment_term_id,
        quotation.id,
    )
    .await?;

    set_quotation_invoice_taxes(&txn, quotation.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for cl in cancelled_lines {
        let line_tax_ids = load_cancelled_line_tax_ids(&txn, cl.id)
            .await
            .map_err(|e| e.to_string())?;
        let line = crate::entities::quotation_invoice_line::ActiveModel {
            quotation_invoice_id: Set(quotation.id),
            product_id: Set(cl.product_id.unwrap_or(0)),
            product_name: Set(cl.product_name.clone()),
            product_remarks: Set(cl.product_remarks.clone()),
            hsn_code: Set(cl.hsn_code),
            sac_code: Set(cl.sac_code),
            variable_types: Set(cl.variable_types.clone()),
            sales_price_formula: Set(cl.sales_price_formula.clone()),
            rate: Set(cl.rate),
            quantity: Set(cl.quantity),
            qty_kind: Set(cl.qty_kind.clone()),
            qty_unit: Set(cl.qty_unit.clone()),
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
        set_quotation_line_taxes(&txn, line.id, &line_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
    }

    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(quotation)
}

struct CancelledLineSnapshot {
    id: i64,
    product_id: Option<i64>,
    product_name: String,
    product_remarks: Option<String>,
    hsn_code: Option<i64>,
    sac_code: Option<i64>,
    variable_types: String,
    sales_price_formula: String,
    rate: Decimal,
    quantity: Decimal,
    qty_kind: String,
    qty_unit: String,
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
            "SELECT id, product_id, product_name, product_remarks, hsn_code, \
             variable_types, sales_price_formula, rate, quantity, qty_kind, qty_unit, \
             variable_values, pre_tax_amount, remarks \
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
                product_name: r.try_get("", "product_name").map_err(|e| e.to_string())?,
                product_remarks: r
                    .try_get("", "product_remarks")
                    .map_err(|e| e.to_string())?,
                hsn_code: r.try_get("", "hsn_code").ok(),
                sac_code: r.try_get("", "sac_code").ok(),
                variable_types: r.try_get("", "variable_types").map_err(|e| e.to_string())?,
                sales_price_formula: r
                    .try_get("", "sales_price_formula")
                    .map_err(|e| e.to_string())?,
                rate: r.try_get("", "rate").map_err(|e| e.to_string())?,
                quantity: r.try_get("", "quantity").map_err(|e| e.to_string())?,
                qty_kind: r.try_get("", "qty_kind").map_err(|e| e.to_string())?,
                qty_unit: r.try_get("", "qty_unit").map_err(|e| e.to_string())?,
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
    product_id: Option<i64>,
    product_name: &str,
    product_remarks: Option<&str>,
    hsn_code: Option<i64>,
    sac_code: Option<i64>,
    variable_types: &str,
    sales_price_formula: &str,
    rate: Decimal,
    quantity: Decimal,
    qty_kind: &str,
    qty_unit: &str,
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
             (cancelled_invoice_id, product_id, product_name, product_remarks, \
              hsn_code, sac_code, variable_types, sales_price_formula, rate, quantity, \
              qty_kind, qty_unit, variable_values, pre_tax_amount, remarks, \
              journal_entry_item_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $17) \
             RETURNING id",
            [
                cancelled_id.into(),
                product_id.into(),
                product_name.into(),
                product_remarks.into(),
                hsn_code.into(),
                sac_code.into(),
                variable_types.into(),
                sales_price_formula.into(),
                rate.into(),
                quantity.into(),
                qty_kind.into(),
                qty_unit.into(),
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
