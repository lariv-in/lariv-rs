//! Draft → Posted, Posted → Cancelled, Cancelled → New draft (purchase_posting.go).

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement,
    TransactionTrait,
};

use lariv_plugin_finance_accounts::logic::custom_source_doc::{
    SubmittedJournalLines, doc_line_indexes, index_of_item, journal_specs_editor_json,
    stored_journal_line_specs,
};
use lariv_plugin_finance_accounts::logic::journal::{
    JournalLineSpec, create_reversing_journal_entry_in_txn, create_source_doc,
    insert_journal_entry, update_source_doc_id,
};
use lariv_plugin_finance_accounts::scope::load_journal_entry_items;
use lariv_plugin_finance_common::decimal;
use lariv_plugin_finance_taxes::entities::tax::Model as TaxModel;
use lariv_plugin_finance_taxes::journal_template::evaluate_tax_for_document;
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::purchase_products::journal_template::{
    self as product_journal_template, ParsedTemplateLine as ProductTemplateLine,
    evaluate_template_lines as evaluate_product_template_lines,
};
use crate::purchase_products::pricing as product_pricing;
use crate::purchase_services::journal_template::{
    self as service_journal_template, ParsedTemplateLine as ServiceTemplateLine,
    evaluate_template_lines as evaluate_service_template_lines,
};
use crate::purchase_services::pricing as service_pricing;

use crate::entities::{
    CancelledPurchaseEntity, DraftPurchaseEntity, DraftPurchaseGoodsLineEntity,
    DraftPurchaseServiceLineEntity, PostedPurchaseEntity, PostedPurchaseGoodsLineEntity,
    PostedPurchaseServiceLineEntity,
};
use crate::entities::{
    cancelled_purchase, draft_purchase, draft_purchase_goods_line, draft_purchase_service_line,
    posted_purchase, posted_purchase_goods_line, posted_purchase_service_line,
};
use crate::logic::draft_payment_term::{
    convert_draft_to_posted_payment_term, copy_posted_payment_term, posted_payment_term_to_draft,
};
use crate::logic::purchase_number::posted_purchase_number;
use crate::logic::tax_assoc::{
    load_cancelled_line_tax_ids, load_cancelled_purchase_tax_ids,
    load_cancelled_service_line_tax_ids, load_draft_line_tax_ids, load_draft_purchase_tax_ids,
    load_draft_service_line_tax_ids, load_posted_line_tax_ids, load_posted_purchase_tax_ids,
    load_posted_service_line_tax_ids, set_cancelled_line_taxes, set_cancelled_purchase_taxes,
    set_cancelled_service_line_taxes, set_draft_line_taxes, set_draft_purchase_taxes,
    set_draft_service_line_taxes, set_posted_line_taxes, set_posted_purchase_taxes,
    set_posted_service_line_taxes,
};
use crate::logic::tax_calculations::{
    PurchaseLinesTotals, document_level_header_taxes, merge_purchase_line_tax_ids,
    purchase_line_amounts, purchase_receivable_grand_total,
};
use crate::scope::find_cancellable_posted;

use crate::entities::cancelled_purchase::PURCHASE_REVERSAL_SOURCE_DOC_TYPE;
use crate::entities::posted_purchase::POSTED_PURCHASE_SOURCE_DOC_TYPE;

struct LineWithTaxes {
    line: draft_purchase_goods_line::Model,
    taxes: Vec<TaxModel>,
}

struct ServiceLineWithTaxes {
    line: draft_purchase_service_line::Model,
    taxes: Vec<TaxModel>,
}

/// Journal lines for one document line, expanded from its product or service template.
struct ExpandedLineJournal {
    specs: Vec<JournalLineSpec>,
    journal_id: i64,
}

fn note_journal(journal_id: &mut Option<i64>, next: i64) -> Result<(), String> {
    if let Some(existing) = *journal_id {
        if existing != next {
            return Err(
                "every product and service on a purchase must use the same journal".to_string(),
            );
        }
    } else {
        *journal_id = Some(next);
    }
    Ok(())
}

async fn expand_product_journal(
    db: &DatabaseConnection,
    product_id: i64,
    variable_values: &str,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
) -> Result<ExpandedLineJournal, String> {
    let product = crate::purchase_products::entities::product::Entity::find_by_id(product_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("product {product_id} not found"))?;
    let Some(template) = product_journal_template::load_template(db, product_id).await? else {
        return Err(format!("{} has no journal entry template", product.name));
    };
    if template.items.is_empty() {
        return Err(format!(
            "{} has no journal entry template lines",
            product.name
        ));
    }
    let parsed: Vec<ProductTemplateLine> = template
        .items
        .iter()
        .map(|item| ProductTemplateLine {
            account_id: item.account_id,
            formula: item.formula.clone(),
        })
        .collect();
    let base_cost = product_pricing::unit_formula_amount(
        &product.variables,
        &product.base_price_formula,
        variable_values,
    )
    .map_err(|e| format!("{} base cost: {e}", product.name))?;
    let sales_price = product_pricing::unit_formula_amount(
        &product.variables,
        &product.sales_price_formula,
        variable_values,
    )
    .map_err(|e| format!("{} sales price: {e}", product.name))?;
    let evaluated = evaluate_product_template_lines(
        &product.variables,
        &parsed,
        variable_values,
        quantity,
        rate,
        pre_tax,
        base_cost,
        sales_price,
    )
    .map_err(|e| format!("{}: {e}", product.name))?;
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

async fn expand_service_journal(
    db: &DatabaseConnection,
    service_id: i64,
    variable_values: &str,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
) -> Result<ExpandedLineJournal, String> {
    let service = crate::purchase_services::entities::service::Entity::find_by_id(service_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("service {service_id} not found"))?;
    let Some(template) = service_journal_template::load_template(db, service_id).await? else {
        return Err(format!("{} has no journal entry template", service.name));
    };
    if template.items.is_empty() {
        return Err(format!(
            "{} has no journal entry template lines",
            service.name
        ));
    }
    let parsed: Vec<ServiceTemplateLine> = template
        .items
        .iter()
        .map(|item| ServiceTemplateLine {
            account_id: item.account_id,
            formula: item.formula.clone(),
        })
        .collect();
    let base_cost = service_pricing::unit_formula_amount(
        &service.variables,
        &service.base_price_formula,
        variable_values,
    )
    .map_err(|e| format!("{} base cost: {e}", service.name))?;
    let sales_price = service_pricing::unit_formula_amount(
        &service.variables,
        &service.sales_price_formula,
        variable_values,
    )
    .map_err(|e| format!("{} sales price: {e}", service.name))?;
    let evaluated = evaluate_service_template_lines(
        &service.variables,
        &parsed,
        variable_values,
        quantity,
        rate,
        pre_tax,
        base_cost,
        sales_price,
    )
    .map_err(|e| format!("{}: {e}", service.name))?;
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

/// Append every tax template line onto the purchase journal.
async fn append_purchase_tax_lines(
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

/// Journal lines confirmed when the purchase was posted.
///
/// Regeneration copies these stored items. Template edits apply to the next post.
pub async fn posted_purchase_journal_lines(
    db: &DatabaseConnection,
    posted_id: i64,
) -> Result<(DateTime<Utc>, Vec<JournalLineSpec>, Vec<usize>), String> {
    let posted = PostedPurchaseEntity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("posted purchase {posted_id} not found"))?;

    let stored = stored_journal_line_specs(db, posted.journal_entry_id).await?;
    let item_ids: Vec<i64> = stored.iter().map(|(id, _)| *id).collect();
    let specs = stored.into_iter().map(|(_, spec)| spec).collect();

    let lines = PostedPurchaseGoodsLineEntity::find()
        .filter(posted_purchase_goods_line::Column::PostedPurchaseId.eq(posted_id))
        .order_by_asc(posted_purchase_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let service_lines = PostedPurchaseServiceLineEntity::find()
        .filter(posted_purchase_service_line::Column::PostedPurchaseId.eq(posted_id))
        .order_by_asc(posted_purchase_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() && service_lines.is_empty() {
        return Err(format!("posted purchase {posted_id} has no lines"));
    }
    let mut indexes = Vec::with_capacity(lines.len() + service_lines.len());
    for line in &lines {
        indexes.push(index_of_item(&item_ids, line.journal_entry_item_id)?);
    }
    for line in &service_lines {
        indexes.push(index_of_item(&item_ids, line.journal_entry_item_id)?);
    }
    Ok((posted.datetime, specs, indexes))
}

/// Point a posted purchase and its lines at a regenerated journal entry.
pub async fn adopt_posted_purchase_journal_entry<C: ConnectionTrait>(
    db: &C,
    posted_id: i64,
    journal_id: i64,
    journal_entry_id: i64,
    item_ids: &[i64],
    line_item_indexes: &[usize],
) -> Result<(), String> {
    let posted = PostedPurchaseEntity::find_by_id(posted_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("posted purchase {posted_id} not found"))?;
    let lines = PostedPurchaseGoodsLineEntity::find()
        .filter(posted_purchase_goods_line::Column::PostedPurchaseId.eq(posted_id))
        .order_by_asc(posted_purchase_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let goods_count = lines.len();
    let service_lines = PostedPurchaseServiceLineEntity::find()
        .filter(posted_purchase_service_line::Column::PostedPurchaseId.eq(posted_id))
        .order_by_asc(posted_purchase_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if goods_count + service_lines.len() != line_item_indexes.len() {
        return Err(format!(
            "posted purchase {posted_id} line count {} does not match {} journal item indexes",
            goods_count + service_lines.len(),
            line_item_indexes.len()
        ));
    }
    for (line, index) in lines.into_iter().zip(line_item_indexes.iter().copied()) {
        let item_id = *item_ids.get(index).ok_or_else(|| {
            format!("posted purchase {posted_id} journal item index {index} is out of range")
        })?;
        let mut am: posted_purchase_goods_line::ActiveModel = line.into();
        am.journal_entry_item_id = Set(item_id);
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await.map_err(|e| e.to_string())?;
    }
    for (line, index) in service_lines
        .into_iter()
        .zip(line_item_indexes.iter().copied().skip(goods_count))
    {
        let item_id = *item_ids.get(index).ok_or_else(|| {
            format!("posted purchase {posted_id} journal item index {index} is out of range")
        })?;
        let mut am: posted_purchase_service_line::ActiveModel = line.into();
        am.journal_entry_item_id = Set(item_id);
        am.updated_at = Set(Some(Utc::now()));
        am.update(db).await.map_err(|e| e.to_string())?;
    }

    let mut am: posted_purchase::ActiveModel = posted.into();
    am.journal_entry_id = Set(journal_entry_id);
    am.journal_id = Set(journal_id);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(|e| e.to_string())?;

    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE cancelled_purchases SET journal_id = $1, updated_at = NOW() WHERE posted_purchase_id = $2",
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
        Some(number) => format!("Draft {number}"),
        None => format!("Draft #{id}"),
    }
}

struct PreparedDraftJournal {
    draft: draft_purchase::Model,
    journal_id: i64,
    specs: Vec<JournalLineSpec>,
    line_item_indexes: Vec<usize>,
    lines_with_taxes: Vec<LineWithTaxes>,
    service_lines_with_taxes: Vec<ServiceLineWithTaxes>,
    header_taxes: Vec<TaxModel>,
    total_payable: Decimal,
}

async fn prepare_draft_journal(
    db: &DatabaseConnection,
    draft_id: i64,
) -> Result<PreparedDraftJournal, String> {
    let draft = DraftPurchaseEntity::find_by_id(draft_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("draft purchase required")?;

    let posted_count = PostedPurchaseEntity::find()
        .filter(posted_purchase::Column::DraftPurchaseId.eq(draft_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if posted_count > 0 {
        return Err("draft already posted".to_string());
    }

    let lines = DraftPurchaseGoodsLineEntity::find()
        .filter(draft_purchase_goods_line::Column::DraftPurchaseId.eq(draft_id))
        .order_by_asc(draft_purchase_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let service_lines = DraftPurchaseServiceLineEntity::find()
        .filter(draft_purchase_service_line::Column::DraftPurchaseId.eq(draft_id))
        .order_by_asc(draft_purchase_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if lines.is_empty() && service_lines.is_empty() {
        return Err("draft has no lines".to_string());
    }

    let header_tax_ids = load_draft_purchase_tax_ids(db, draft_id)
        .await
        .map_err(|e| e.to_string())?;
    let header_taxes = load_taxes_by_ids(db, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    let mut lines_with_taxes = Vec::with_capacity(lines.len());
    let mut service_lines_with_taxes = Vec::with_capacity(service_lines.len());
    let mut specs = Vec::new();
    let mut line_item_indexes = Vec::with_capacity(lines.len() + service_lines.len());
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
            line.product_id,
            &line.variable_values,
            line.quantity,
            line.rate,
            line.pre_tax_amount,
        )
        .await?;
        note_journal(&mut journal_id, expanded.journal_id)?;
        line_item_indexes.push(specs.len());
        specs.extend(expanded.specs);
        lines_with_taxes.push(LineWithTaxes { line, taxes });
    }
    for line in service_lines {
        let tax_ids = load_draft_service_line_tax_ids(db, line.id)
            .await
            .map_err(|e| e.to_string())?;
        let taxes = load_taxes_by_ids(db, &tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        let expanded = expand_service_journal(
            db,
            line.service_id,
            &line.variable_values,
            line.quantity,
            line.rate,
            line.pre_tax_amount,
        )
        .await?;
        note_journal(&mut journal_id, expanded.journal_id)?;
        line_item_indexes.push(specs.len());
        specs.extend(expanded.specs);
        service_lines_with_taxes.push(ServiceLineWithTaxes { line, taxes });
    }
    let journal_id = journal_id.ok_or("draft has no lines")?;
    let tax_inputs: Vec<(Decimal, Vec<TaxModel>)> = lines_with_taxes
        .iter()
        .map(|lwt| (lwt.line.pre_tax_amount, lwt.taxes.clone()))
        .chain(
            service_lines_with_taxes
                .iter()
                .map(|lwt| (lwt.line.pre_tax_amount, lwt.taxes.clone())),
        )
        .collect();
    append_purchase_tax_lines(db, &mut specs, &tax_inputs, &header_taxes, journal_id).await?;

    let mut totals = PurchaseLinesTotals::default();
    let mut all_line_tax_ids = HashSet::new();
    for (pre_tax_amount, taxes) in lines_with_taxes
        .iter()
        .map(|lwt| (lwt.line.pre_tax_amount, &lwt.taxes))
        .chain(
            service_lines_with_taxes
                .iter()
                .map(|lwt| (lwt.line.pre_tax_amount, &lwt.taxes)),
        )
    {
        merge_purchase_line_tax_ids(&mut all_line_tax_ids, taxes);
        let (untaxed, levied, withholding, _) = purchase_line_amounts(pre_tax_amount, taxes);
        totals.untaxed_subtotal = decimal::dec_sum(totals.untaxed_subtotal, untaxed);
        totals.lines_levied = decimal::dec_sum(totals.lines_levied, levied);
        totals.lines_withholding = decimal::dec_sum(totals.lines_withholding, withholding);
    }
    let total_payable = purchase_receivable_grand_total(&totals, &header_taxes, &all_line_tax_ids);
    Ok(PreparedDraftJournal {
        draft,
        journal_id,
        specs,
        line_item_indexes,
        lines_with_taxes,
        service_lines_with_taxes,
        header_taxes,
        total_payable,
    })
}

pub async fn draft_new_posted(
    db: &DatabaseConnection,
    draft_id: i64,
    posted_at: DateTime<Utc>,
    tz: &str,
) -> Result<posted_purchase::Model, String> {
    draft_new_posted_with_lines(db, draft_id, posted_at, tz, None).await
}

pub async fn draft_new_posted_with_lines(
    db: &DatabaseConnection,
    draft_id: i64,
    posted_at: DateTime<Utc>,
    tz: &str,
    submitted: Option<SubmittedJournalLines>,
) -> Result<posted_purchase::Model, String> {
    let PreparedDraftJournal {
        draft,
        journal_id,
        mut specs,
        mut line_item_indexes,
        lines_with_taxes,
        service_lines_with_taxes,
        header_taxes,
        total_payable,
    } = prepare_draft_journal(db, draft_id).await?;
    if let Some(submitted) = submitted {
        line_item_indexes = doc_line_indexes(line_item_indexes.len(), &submitted.anchors);
        specs = submitted.lines;
    }

    let number = posted_purchase_number(db, &draft).await?;
    let dup_posted = PostedPurchaseEntity::find()
        .filter(posted_purchase::Column::Number.eq(&number))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if dup_posted > 0 {
        return Err(format!(
            "purchase number {number} is already used by another posted purchase"
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
    let doc_id = create_source_doc(&txn, POSTED_PURCHASE_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;
    let (je_id, je_items) =
        insert_journal_entry(&txn, source_doc_datetime, journal_id, doc_id, &specs)
            .await
            .map_err(|e| e.to_string())?;

    let now = Utc::now();
    let payment_term =
        convert_draft_to_posted_payment_term(&txn, draft.id, draft.datetime, total_payable, tz)
            .await?;
    let posted_am = posted_purchase::ActiveModel {
        draft_purchase_id: Set(draft.id),
        posted_at: Set(Some(posted_at)),
        number: Set(number),
        reference: Set(draft.reference.clone()),
        payment_reference: Set(draft.payment_reference.clone()),
        bank_account: Set(draft.bank_account.clone()),
        remarks: Set(draft.remarks.clone()),
        journal_id: Set(journal_id),
        datetime: Set(draft.datetime),
        delivery_date: Set(draft.delivery_date),
        vendor_id: Set(draft.vendor_id),
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
    set_posted_purchase_taxes(&txn, posted.id, &header_tax_ids_only)
        .await
        .map_err(|e| e.to_string())?;

    for (i, lwt) in lines_with_taxes.iter().enumerate() {
        let item_idx = *line_item_indexes
            .get(i)
            .ok_or("internal error: journal item index")?;
        let expense_item_id = je_items
            .get(item_idx)
            .map(|it| it.id)
            .ok_or("internal error: journal item index")?;
        let pl_am = posted_purchase_goods_line::ActiveModel {
            posted_purchase_id: Set(posted.id),
            product_id: Set(lwt.line.product_id),
            rate: Set(lwt.line.rate),
            quantity: Set(lwt.line.quantity),
            variable_values: Set(lwt.line.variable_values.clone()),
            pre_tax_amount: Set(lwt.line.pre_tax_amount),
            remarks: Set(lwt.line.remarks.clone()),
            journal_entry_item_id: Set(expense_item_id),
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
    for (i, lwt) in service_lines_with_taxes.iter().enumerate() {
        let item_idx = *line_item_indexes
            .get(lines_with_taxes.len() + i)
            .ok_or("internal error: journal item index")?;
        let expense_item_id = je_items
            .get(item_idx)
            .map(|it| it.id)
            .ok_or("internal error: journal item index")?;
        let pl_am = posted_purchase_service_line::ActiveModel {
            posted_purchase_id: Set(posted.id),
            service_id: Set(lwt.line.service_id),
            rate: Set(lwt.line.rate),
            quantity: Set(lwt.line.quantity),
            variable_values: Set(lwt.line.variable_values.clone()),
            pre_tax_amount: Set(lwt.line.pre_tax_amount),
            remarks: Set(lwt.line.remarks.clone()),
            journal_entry_item_id: Set(expense_item_id),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        let pl = pl_am.insert(&txn).await.map_err(|e| e.to_string())?;
        let tax_ids: Vec<i64> = lwt.taxes.iter().map(|t| t.id).collect();
        set_posted_service_line_taxes(&txn, pl.id, &tax_ids)
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
) -> Result<cancelled_purchase::Model, String> {
    let posted = find_cancellable_posted(db, posted_id)
        .await
        .ok_or("posted purchase is not cancellable")?;

    let posted_lines = PostedPurchaseGoodsLineEntity::find()
        .filter(posted_purchase_goods_line::Column::PostedPurchaseId.eq(posted_id))
        .order_by_asc(posted_purchase_goods_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let header_tax_ids = load_posted_purchase_tax_ids(db, posted_id)
        .await
        .map_err(|e| e.to_string())?;

    let at = if at.timestamp() == 0 { Utc::now() } else { at };

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let (reversal_doc_id, rev_je_id) = create_reversing_journal_entry_in_txn(
        &txn,
        posted.journal_entry_id,
        at,
        PURCHASE_REVERSAL_SOURCE_DOC_TYPE,
    )
    .await
    .map_err(|e| e.to_string())?;
    txn.commit().await.map_err(|e| e.to_string())?;

    let mut orig_items: Vec<_> = load_journal_entry_items(db, posted.journal_entry_id)
        .await
        .into_iter()
        .map(|(item, _)| item)
        .collect();
    let mut rev_items: Vec<_> = load_journal_entry_items(db, rev_je_id)
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
    let cam = cancelled_purchase::ActiveModel {
        posted_purchase_id: Set(posted.id),
        posted_at: Set(posted.posted_at),
        cancelled_at: Set(Some(at)),
        number: Set(posted.number.clone()),
        reference: Set(posted.reference.clone()),
        payment_reference: Set(posted.payment_reference.clone()),
        bank_account: Set(posted.bank_account.clone()),
        remarks: Set(posted.remarks.clone()),
        journal_id: Set(posted.journal_id),
        datetime: Set(posted.datetime),
        delivery_date: Set(posted.delivery_date),
        vendor_id: Set(posted.vendor_id),
        reversed_journal_entry_id: Set(rev_je_id),
        reason: Set(Some(reason)),
        posted_payment_term_id: Set(payment_term_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    };
    let cancelled = cam.insert(&txn).await.map_err(|e| e.to_string())?;
    update_source_doc_id(&txn, reversal_doc_id, cancelled.id)
        .await
        .map_err(|e| e.to_string())?;

    set_cancelled_purchase_taxes(&txn, cancelled.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for pl in posted_lines {
        let rev_id = orig_to_rev.get(&pl.journal_entry_item_id).ok_or_else(|| {
            format!(
                "could not map journal line for posted purchase line {}",
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
    let posted_service_lines = PostedPurchaseServiceLineEntity::find()
        .filter(posted_purchase_service_line::Column::PostedPurchaseId.eq(posted_id))
        .order_by_asc(posted_purchase_service_line::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    for pl in posted_service_lines {
        let rev_id = orig_to_rev.get(&pl.journal_entry_item_id).ok_or_else(|| {
            format!(
                "could not map journal line for posted purchase service line {}",
                pl.id
            )
        })?;
        let line_tax_ids = load_posted_service_line_tax_ids(&txn, pl.id)
            .await
            .map_err(|e| e.to_string())?;
        let cl_id = insert_cancelled_service_line(
            &txn,
            cancelled.id,
            pl.service_id,
            pl.rate,
            pl.quantity,
            &pl.variable_values,
            pl.pre_tax_amount,
            pl.remarks.clone(),
            *rev_id,
            now,
        )
        .await?;
        set_cancelled_service_line_taxes(&txn, cl_id, &line_tax_ids)
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
) -> Result<draft_purchase::Model, String> {
    let cancelled = CancelledPurchaseEntity::find_by_id(cancelled_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("cancelled purchase required")?;

    let header_tax_ids = load_cancelled_purchase_tax_ids(db, cancelled_id)
        .await
        .map_err(|e| e.to_string())?;
    let cancelled_lines = load_cancelled_purchase_goods_lines(db, cancelled_id).await?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now();
    let draft = draft_purchase::ActiveModel {
        number: Set(None),
        reference: Set(cancelled.reference.clone()),
        payment_reference: Set(cancelled.payment_reference.clone()),
        bank_account: Set(cancelled.bank_account.clone()),
        remarks: Set(cancelled.remarks.clone()),
        datetime: Set(cancelled.datetime),
        delivery_date: Set(cancelled.delivery_date),
        vendor_id: Set(cancelled.vendor_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    posted_payment_term_to_draft(&txn, cancelled.posted_payment_term_id, draft.id).await?;

    set_draft_purchase_taxes(&txn, draft.id, &header_tax_ids)
        .await
        .map_err(|e| e.to_string())?;

    for cl in cancelled_lines {
        let line_tax_ids = load_cancelled_line_tax_ids(&txn, cl.id)
            .await
            .map_err(|e| e.to_string())?;
        let line = draft_purchase_goods_line::ActiveModel {
            draft_purchase_id: Set(draft.id),
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
    let service_rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, service_id, rate, quantity, variable_values, pre_tax_amount, remarks \
             FROM cancelled_purchase_service_lines \
             WHERE cancelled_purchase_id = $1 ORDER BY id ASC",
            [cancelled_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    for row in service_rows {
        let id: i64 = row.try_get("", "id").map_err(|e| e.to_string())?;
        let line_tax_ids = load_cancelled_service_line_tax_ids(&txn, id)
            .await
            .map_err(|e| e.to_string())?;
        let line = draft_purchase_service_line::ActiveModel {
            draft_purchase_id: Set(draft.id),
            service_id: Set(row.try_get("", "service_id").map_err(|e| e.to_string())?),
            rate: Set(row.try_get("", "rate").map_err(|e| e.to_string())?),
            quantity: Set(row.try_get("", "quantity").map_err(|e| e.to_string())?),
            variable_values: Set(row
                .try_get("", "variable_values")
                .map_err(|e| e.to_string())?),
            pre_tax_amount: Set(row
                .try_get("", "pre_tax_amount")
                .map_err(|e| e.to_string())?),
            remarks: Set(row.try_get("", "remarks").map_err(|e| e.to_string())?),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;
        set_draft_service_line_taxes(&txn, line.id, &line_tax_ids)
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

async fn load_cancelled_purchase_goods_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Result<Vec<CancelledLineSnapshot>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id, product_id, rate, quantity, variable_values, pre_tax_amount, remarks \
             FROM cancelled_purchase_goods_lines \
             WHERE cancelled_purchase_id = $1 ORDER BY id ASC",
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
            "INSERT INTO cancelled_purchase_goods_lines \
             (cancelled_purchase_id, product_id, rate, quantity, variable_values, pre_tax_amount, remarks, journal_entry_item_id, created_at, updated_at) \
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

async fn insert_cancelled_service_line<C: ConnectionTrait>(
    db: &C,
    cancelled_id: i64,
    service_id: i64,
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
            "INSERT INTO cancelled_purchase_service_lines \
             (cancelled_purchase_id, service_id, rate, quantity, variable_values, pre_tax_amount, remarks, journal_entry_item_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9) RETURNING id",
            [
                cancelled_id.into(),
                service_id.into(),
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
        .ok_or_else(|| "insert cancelled service line failed".to_string())?;
    row.try_get("", "id").map_err(|e| e.to_string())
}

/// Negate the posted purchase journal. Indexes are the stored anchor of each goods or service line.
pub async fn cancelled_reversal_journal_lines(
    db: &DatabaseConnection,
    cancelled_id: i64,
) -> Result<(DateTime<Utc>, Vec<JournalLineSpec>, Vec<usize>), String> {
    let cancelled = CancelledPurchaseEntity::find_by_id(cancelled_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("cancelled purchase {cancelled_id} not found"))?;
    let (posted_dt, lines, indexes) =
        posted_purchase_journal_lines(db, cancelled.posted_purchase_id).await?;
    let lines = lines
        .into_iter()
        .map(|line| JournalLineSpec {
            account_id: line.account_id,
            amount: decimal::dec_neg(line.amount),
        })
        .collect();
    Ok((cancelled.cancelled_at.unwrap_or(posted_dt), lines, indexes))
}

/// Point a cancelled purchase's reversal entry and lines at a regenerated journal.
pub async fn adopt_cancelled_purchase_journal_entry<C: ConnectionTrait>(
    db: &C,
    cancelled_id: i64,
    journal_entry_id: i64,
    item_ids: &[i64],
    line_item_indexes: &[usize],
) -> Result<(), String> {
    let line_ids = query_i64s(
        db,
        "SELECT id FROM cancelled_purchase_goods_lines WHERE cancelled_purchase_id = $1 ORDER BY id ASC",
        cancelled_id,
    )
    .await?;
    if line_ids.len() != line_item_indexes.len() {
        return Err(format!(
            "cancelled purchase {cancelled_id} line count {} does not match {} journal item indexes",
            line_ids.len(),
            line_item_indexes.len()
        ));
    }
    for (line_id, index) in line_ids.into_iter().zip(line_item_indexes.iter().copied()) {
        let item_id = *item_ids.get(index).ok_or_else(|| {
            format!("cancelled purchase {cancelled_id} journal item index {index} is out of range")
        })?;
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE cancelled_purchase_goods_lines SET journal_entry_item_id = $1, updated_at = NOW() WHERE id = $2",
            [item_id.into(), line_id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    }
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE cancelled_purchases SET reversed_journal_entry_id = $1, updated_at = NOW() WHERE id = $2",
        [journal_entry_id.into(), cancelled_id.into()],
    ))
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn query_i64s<C: ConnectionTrait>(db: &C, sql: &str, id: i64) -> Result<Vec<i64>, String> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            [id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    rows.into_iter()
        .map(|row| row.try_get("", "id").map_err(|e| e.to_string()))
        .collect()
}
