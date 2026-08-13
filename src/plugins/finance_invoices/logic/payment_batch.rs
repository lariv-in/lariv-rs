//! Batch payment: one bank receipt clears multiple posted invoices in a single journal entry.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::plugins::finance_accounts::{
    logic::journal::debit_balance_type,
    logic::journal::{
        JournalLineSpec, create_source_doc, insert_journal_entry, update_source_doc_id,
    },
    scope::load_journal_display_label,
    validate_leaf_account_balance_type,
};
use crate::plugins::finance_common::decimal;
use crate::plugins::finance_products::preferences::optional_i64;
use crate::plugins::finance_taxes::scope::load_taxes_by_ids;

use crate::plugins::finance_invoices::entities::payment_batch::PAYMENT_BATCH_SOURCE_DOC_TYPE;
use crate::plugins::finance_invoices::entities::{payment, payment_batch, posted_invoice};
use crate::plugins::finance_invoices::logic::payment::{
    build_payment_lines_for_allocation, record_payment_settlement, validate_payment_allocation,
};
use crate::plugins::finance_invoices::logic::preferences::{
    load_payment_preferences, validate_payment_preferences_for_create,
};
use crate::plugins::finance_invoices::logic::tax_assoc::set_payment_taxes;
use crate::plugins::finance_invoices::logic::tax_calculations::{
    payment_withholding_base, validate_payment_taxes,
};

#[derive(Debug)]
pub struct BatchAllocation {
    pub posted_invoice_id: i64,
    pub amount: Decimal,
    pub withholding_tax_ids: Vec<i64>,
}

pub struct CreatePaymentBatchInput {
    pub datetime: DateTime<Utc>,
    pub account_id: Option<i64>,
    pub allocations: Vec<BatchAllocation>,
}

pub struct CreatePaymentBatchResult {
    pub batch: payment_batch::Model,
    pub payment_ids: Vec<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct AllocationJsonRow {
    posted_invoice_id: i64,
    amount: String,
    #[serde(default)]
    tax_ids: Vec<i64>,
}

pub fn parse_batch_allocations_json(json: &str) -> Result<Vec<BatchAllocation>, String> {
    let rows: Vec<AllocationJsonRow> =
        serde_json::from_str(json).map_err(|_| "invalid allocations JSON".to_string())?;
    if rows.len() < 2 {
        return Err("batch payment requires at least two invoices".to_string());
    }
    let mut seen = HashSet::new();
    let mut allocations = Vec::with_capacity(rows.len());
    for row in rows {
        if row.posted_invoice_id <= 0 {
            return Err("invalid posted invoice id".to_string());
        }
        if !seen.insert(row.posted_invoice_id) {
            return Err("duplicate invoice in batch".to_string());
        }
        let amount =
            crate::plugins::finance_invoices::logic::payment::parse_payment_amount(&row.amount)?;
        allocations.push(BatchAllocation {
            posted_invoice_id: row.posted_invoice_id,
            amount,
            withholding_tax_ids: row.tax_ids,
        });
    }
    Ok(allocations)
}

fn validate_same_journal(posted: &[posted_invoice::Model]) -> Result<i64, String> {
    let journal_id = posted[0].journal_id;
    let mut journal_names: HashMap<i64, String> = HashMap::new();
    for inv in posted {
        if inv.journal_id != journal_id {
            journal_names
                .entry(inv.journal_id)
                .or_insert_with(|| inv.journal_id.to_string());
            journal_names
                .entry(journal_id)
                .or_insert_with(|| journal_id.to_string());
        }
    }
    if journal_names.len() > 1 {
        return Err(format!(
            "invoices span multiple journals (ids: {})",
            journal_names
                .keys()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(journal_id)
}

pub async fn create_payment_batch(
    db: &DatabaseConnection,
    input: CreatePaymentBatchInput,
) -> Result<CreatePaymentBatchResult, String> {
    if input.allocations.len() < 2 {
        return Err("batch payment requires at least two invoices".to_string());
    }

    let mut seen = HashSet::new();
    for alloc in &input.allocations {
        if !seen.insert(alloc.posted_invoice_id) {
            return Err("duplicate invoice in batch".to_string());
        }
    }

    let payment_prefs = load_payment_preferences(db).await;
    validate_payment_preferences_for_create(db, &payment_prefs).await?;

    let account_id = input
        .account_id
        .filter(|id| *id > 0)
        .unwrap_or_else(|| optional_i64(payment_prefs.payment_account_id));
    validate_leaf_account_balance_type(db, account_id, debit_balance_type(), "payment account")
        .await
        .map_err(|e| e.to_string())?;

    let posted_ids: Vec<i64> = input
        .allocations
        .iter()
        .map(|a| a.posted_invoice_id)
        .collect();
    let posted_models = posted_invoice::Entity::find()
        .filter(posted_invoice::Column::Id.is_in(posted_ids.clone()))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    if posted_models.len() != posted_ids.len() {
        return Err("one or more posted invoices not found".to_string());
    }

    let journal_id = match validate_same_journal(&posted_models) {
        Ok(id) => id,
        Err(_) => {
            let mut journal_ids: HashSet<i64> = HashSet::new();
            for inv in &posted_models {
                journal_ids.insert(inv.journal_id);
            }
            let mut labels: Vec<String> = Vec::new();
            for jid in journal_ids {
                let label = load_journal_display_label(db, Some(jid)).await;
                labels.push(label);
            }
            labels.sort();
            return Err(format!(
                "all invoices in a batch must share the same journal (found: {})",
                labels.join(", ")
            ));
        }
    };

    struct PreparedAllocation {
        posted: posted_invoice::Model,
        settlement: Decimal,
        is_full: bool,
        tax_ids: Vec<i64>,
        journal_lines: Vec<JournalLineSpec>,
    }

    let mut prepared = Vec::with_capacity(input.allocations.len());
    let mut total_bank = Decimal::ZERO;
    let mut total_settlement = Decimal::ZERO;

    for alloc in &input.allocations {
        let (posted, inv_total, untaxed_subtotal, is_full) =
            validate_payment_allocation(db, alloc.posted_invoice_id, alloc.amount).await?;
        let taxes = load_taxes_by_ids(db, &alloc.withholding_tax_ids)
            .await
            .map_err(|e| e.to_string())?;
        validate_payment_taxes(&taxes)?;

        let settlement = decimal::normalize(alloc.amount);
        let withholding_base = payment_withholding_base(settlement, inv_total, untaxed_subtotal);
        let (bank_amt, journal_lines) =
            build_payment_lines_for_allocation(&posted, settlement, withholding_base, &taxes)?;

        total_bank = decimal::dec_sum(total_bank, bank_amt);
        total_settlement = decimal::dec_sum(total_settlement, settlement);

        prepared.push(PreparedAllocation {
            posted,
            settlement,
            is_full,
            tax_ids: alloc.withholding_tax_ids.clone(),
            journal_lines,
        });
    }

    let now = Utc::now();
    let dt = if input.datetime.timestamp() == 0 {
        now
    } else {
        input.datetime
    };

    let txn = db.begin().await.map_err(|e| e.to_string())?;

    let doc_id = create_source_doc(&txn, PAYMENT_BATCH_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;

    let mut lines = vec![JournalLineSpec {
        account_id,
        amount: total_bank,
    }];
    for prep in &prepared {
        lines.extend(prep.journal_lines.clone());
    }

    let (je_id, _) = insert_journal_entry(&txn, now, journal_id, doc_id, &lines)
        .await
        .map_err(|e| e.to_string())?;

    let batch = payment_batch::ActiveModel {
        datetime: Set(dt),
        account_id: Set(account_id),
        journal_entry_id: Set(je_id),
        total_amount: Set(total_settlement),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| e.to_string())?;

    let mut payment_ids = Vec::with_capacity(prepared.len());
    for prep in prepared {
        let pay = payment::ActiveModel {
            posted_invoice_id: Set(prep.posted.id),
            amount: Set(prep.settlement),
            account_id: Set(account_id),
            datetime: Set(dt),
            journal_entry_id: Set(je_id),
            payment_batch_id: Set(Some(batch.id)),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(|e| e.to_string())?;

        set_payment_taxes(&txn, pay.id, &prep.tax_ids)
            .await
            .map_err(|e| e.to_string())?;

        let _settlement_id =
            record_payment_settlement(&txn, pay.id, prep.posted.id, prep.is_full).await?;

        payment_ids.push(pay.id);
    }

    update_source_doc_id(&txn, doc_id, batch.id)
        .await
        .map_err(|e| e.to_string())?;

    txn.commit().await.map_err(|e| e.to_string())?;

    Ok(CreatePaymentBatchResult { batch, payment_ids })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::finance_taxes::entities::{TaxKind, tax};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn sample_posted(id: i64, ar_id: i64) -> posted_invoice::Model {
        posted_invoice::Model {
            id,
            created_at: None,
            updated_at: None,
            draft_invoice_id: 0,
            posted_at: None,
            number: format!("INV-{id}"),
            reference: None,
            payment_reference: None,
            bank_account: None,
            account_receivable_id: ar_id,
            account_revenue_id: 0,
            account_tax_payable_id: 0,
            journal_id: 1,
            datetime: Utc::now(),
            customer_id: 0,
            journal_entry_id: 0,
        }
    }

    fn withholding_tax(id: i64, pct: Decimal, acct: i64) -> tax::Model {
        tax::Model {
            id,
            created_at: None,
            updated_at: None,
            name: format!("WH {id}"),
            percentage: pct,
            tax_type: TaxKind::Withholding,
            account_id: Some(acct),
        }
    }

    #[test]
    fn parse_batch_allocations_json_requires_two_rows() {
        let err = parse_batch_allocations_json(r#"[{"posted_invoice_id":1,"amount":"100"}]"#)
            .unwrap_err();
        assert!(err.contains("at least two"));
    }

    #[test]
    fn parse_batch_allocations_json_rejects_duplicate_invoice() {
        let json = r#"[
            {"posted_invoice_id":1,"amount":"100","tax_ids":[]},
            {"posted_invoice_id":1,"amount":"50","tax_ids":[]}
        ]"#;
        let err = parse_batch_allocations_json(json).unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn parse_batch_allocations_json_ok() {
        let json = r#"[
            {"posted_invoice_id":1,"amount":"100.00","tax_ids":[2]},
            {"posted_invoice_id":3,"amount":"50","tax_ids":[]}
        ]"#;
        let allocs = parse_batch_allocations_json(json).unwrap();
        assert_eq!(allocs.len(), 2);
        assert_eq!(allocs[0].posted_invoice_id, 1);
        assert_eq!(allocs[0].amount, d("100.00"));
        assert_eq!(allocs[0].withholding_tax_ids, vec![2]);
    }

    #[test]
    fn batch_journal_lines_balance_with_withholding() {
        let posted1 = sample_posted(1, 100);
        let posted2 = sample_posted(2, 200);
        let taxes = vec![withholding_tax(10, d("10"), 300)];

        // Withholding base is untaxed (100), not GST-inclusive settlement (118).
        let (bank1, lines1) =
            build_payment_lines_for_allocation(&posted1, d("118"), d("100"), &taxes).unwrap();
        let (bank2, lines2) =
            build_payment_lines_for_allocation(&posted2, d("200"), d("200"), &[]).unwrap();

        let mut lines = vec![JournalLineSpec {
            account_id: 50,
            amount: decimal::dec_sum(bank1, bank2),
        }];
        lines.extend(lines1);
        lines.extend(lines2);

        let balance: Decimal = lines.iter().map(|l| l.amount).sum();
        assert!(decimal::dec_is_zero(balance));
        // 10% of untaxed 100 = 10; bank = 118 - 10 = 108
        assert_eq!(bank1, d("108"));
        assert_eq!(bank2, d("200"));
    }

    #[test]
    fn validate_same_journal_rejects_mixed() {
        let mut a = sample_posted(1, 100);
        let mut b = sample_posted(2, 200);
        b.journal_id = 2;
        let err = validate_same_journal(&[a.clone(), b.clone()]).unwrap_err();
        assert!(err.contains("multiple journals"));

        a.journal_id = 5;
        b.journal_id = 5;
        assert_eq!(validate_same_journal(&[a, b]).unwrap(), 5);
    }

    /// Manual verification checklist (run against a dev environment):
    /// 1. Post 2 invoices (same journal, different customers) with open balances.
    /// 2. On Posted tab, select both → Pay selected → full balances, save batch.
    /// 3. Verify one journal entry in accounts; both invoices leave Posted tab.
    /// 4. Batch detail shows 2 child payments linked to the shared journal entry.
    /// 5. Retry with per-invoice withholding; bank debit equals net of withholdings.
    /// 6. Confirm mixed journals, over-allocation, cancelled invoice, and single-invoice batch are rejected.
    #[test]
    fn manual_verification_checklist_documented() {}
}
