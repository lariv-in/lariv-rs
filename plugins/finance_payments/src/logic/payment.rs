//! Record a customer payment and its journal entry.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, EntityTrait,
    TransactionTrait,
};

use lariv_formula::decimal::{self, parse_decimal};
use lariv_plugin_customer::entities::customer::Entity as CustomerEntity;
use lariv_plugin_finance_accounts::logic::custom_source_doc::journal_specs_editor_json;
use lariv_plugin_finance_accounts::logic::journal::{
    JournalLineSpec, create_source_doc, insert_journal_entry, update_source_doc_id,
};
use lariv_plugin_finance_taxes::entities::tax::{self, TaxKind};
use lariv_plugin_finance_taxes::scope::load_taxes_by_ids;

use crate::entities::payment::{self, PAYMENT_SOURCE_DOC_TYPE};
use crate::logic::preferences::{
    load_payment_preferences, validate_payment_preferences_for_create,
};
use crate::logic::tax_assoc::set_payment_taxes;

pub struct CreatePaymentInput {
    pub customer_id: i64,
    pub amount: Decimal,
    pub datetime: DateTime<Utc>,
    pub withholding_tax_ids: Vec<i64>,
    /// `None` evaluates the payment template. `Some` writes these lines.
    pub journal_lines: Option<Vec<JournalLineSpec>>,
    /// Untaxed base the withholding percent applies to. Standalone payments use the amount.
    pub withholding_base: Option<Decimal>,
    /// Trimmed payment number. Blank or `None` assigns one from Payment number format.
    pub number: Option<String>,
    pub is_gst_advance: bool,
    pub gst_pos: Option<String>,
    pub gst_tax_ids: Vec<i64>,
    pub adjustments: Vec<(chrono::NaiveDate, Decimal)>,
}

pub fn parse_payment_amount(s: &str) -> Result<Decimal, String> {
    parse_decimal(s).ok_or_else(|| "invalid amount".to_string())
}

pub fn parse_withholding_tax_ids(s: &str) -> Vec<i64> {
    s.split(',')
        .filter_map(|p| p.trim().parse().ok())
        .filter(|id| *id > 0)
        .collect()
}

fn validate_payment_taxes(taxes: &[tax::Model]) -> Result<(), String> {
    for t in taxes {
        if t.tax_type != TaxKind::Withholding {
            let name = if t.name.trim().is_empty() {
                format!("#{}", t.id)
            } else {
                t.name.clone()
            };
            return Err(format!(
                "levied tax {name} cannot be applied on a payment; use withholding taxes only"
            ));
        }
    }
    Ok(())
}

fn withholding_total(withholding_base: Decimal, taxes: &[tax::Model]) -> Decimal {
    let pct: Decimal = taxes
        .iter()
        .filter(|t| t.tax_type == TaxKind::Withholding)
        .map(|t| t.percentage)
        .sum();
    if decimal::dec_is_zero(withholding_base) || decimal::dec_is_zero(pct) {
        return Decimal::ZERO;
    }
    decimal::dec_mul(withholding_base, pct / Decimal::from(100))
}

/// Payment template lines plus withholding lines. Posts on the template journal.
pub async fn build_payment_lines<C: ConnectionTrait>(
    db: &C,
    settlement: Decimal,
    withholding_base: Decimal,
    taxes: &[tax::Model],
) -> Result<Vec<JournalLineSpec>, String> {
    let settlement = decimal::normalize(settlement);
    let withholding_base = decimal::normalize(withholding_base);
    let withheld = withholding_total(withholding_base, taxes);
    let (journal_id, mut lines) =
        crate::payment_journal_template::evaluate_payment_template(db, settlement, withheld)
            .await?;
    for tax in taxes.iter().filter(|t| t.tax_type == TaxKind::Withholding) {
        let evaluated = lariv_plugin_finance_taxes::journal_template::evaluate_withholding_lines(
            db,
            tax,
            withholding_base,
            journal_id,
        )
        .await?;
        lines.extend(evaluated.into_iter().map(|line| JournalLineSpec {
            account_id: line.account_id,
            amount: line.amount,
        }));
    }
    Ok(lines)
}

/// Editor JSON for the payment template and withholding lines at this amount.
pub async fn suggest_payment_lines_json(
    db: &DatabaseConnection,
    amount: Decimal,
    withholding_base: Decimal,
    withholding_tax_ids: &[i64],
) -> Result<String, String> {
    let taxes = load_taxes_by_ids(db, withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    validate_payment_taxes(&taxes)?;
    let lines = build_payment_lines(db, amount, withholding_base, &taxes).await?;
    journal_specs_editor_json(db, &lines, &[], "payment-line").await
}

/// Point a payment at the journal entry created for its source document.
pub async fn adopt_payment_journal_entry<C: ConnectionTrait>(
    db: &C,
    payment_id: i64,
    journal_entry_id: i64,
) -> Result<(), String> {
    let pay = payment::Entity::find_by_id(payment_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("payment {payment_id} not found"))?;
    let mut am: payment::ActiveModel = pay.into();
    am.journal_entry_id = Set(journal_entry_id);
    am.updated_at = Set(Some(Utc::now()));
    am.update(db).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn payment_journal_lines(
    db: &DatabaseConnection,
    payment_id: i64,
) -> Result<(DateTime<Utc>, Vec<JournalLineSpec>), String> {
    use lariv_plugin_finance_accounts::logic::custom_source_doc::stored_journal_line_specs;

    let pay = payment::Entity::find_by_id(payment_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("payment {payment_id} not found"))?;
    let stored = stored_journal_line_specs(db, pay.journal_entry_id).await?;
    let lines = stored.into_iter().map(|(_, spec)| spec).collect();
    Ok((pay.datetime, lines))
}

pub struct NewPayment {
    pub customer_id: i64,
    pub amount: Decimal,
    pub datetime: DateTime<Utc>,
    pub journal_id: i64,
    pub lines: Vec<JournalLineSpec>,
    pub withholding_tax_ids: Vec<i64>,
    pub number: Option<String>,
    pub is_gst_advance: bool,
    pub gst_pos: Option<String>,
    pub gst_tax_ids: Vec<i64>,
    pub adjustments: Vec<(chrono::NaiveDate, Decimal)>,
}

pub async fn prepare_payment(
    db: &DatabaseConnection,
    input: CreatePaymentInput,
) -> Result<NewPayment, String> {
    if input.customer_id <= 0 {
        return Err("customer is required".to_string());
    }
    if input.amount <= Decimal::ZERO {
        return Err("amount must be positive".to_string());
    }
    let customer = CustomerEntity::find_by_id(input.customer_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?;
    if customer.is_none() {
        return Err("customer not found".to_string());
    }

    load_payment_preferences(db).await;
    validate_payment_preferences_for_create(db).await?;

    let taxes = load_taxes_by_ids(db, &input.withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    validate_payment_taxes(&taxes)?;

    let now = Utc::now();
    let dt = if input.datetime.timestamp() == 0 {
        now
    } else {
        input.datetime
    };
    let settlement = decimal::normalize(input.amount);
    let withholding_base = decimal::normalize(input.withholding_base.unwrap_or(settlement));
    let lines = if let Some(lines) = input.journal_lines {
        lines
    } else {
        build_payment_lines(db, settlement, withholding_base, &taxes).await?
    };
    let journal_id = crate::payment_journal_template::load_template(db)
        .await?
        .ok_or_else(|| "payment preferences have no journal entry template".to_string())?
        .journal_id;
    Ok(NewPayment {
        customer_id: input.customer_id,
        amount: settlement,
        datetime: dt,
        journal_id,
        lines,
        withholding_tax_ids: input.withholding_tax_ids,
        number: input.number,
        is_gst_advance: input.is_gst_advance,
        gst_pos: input.gst_pos,
        gst_tax_ids: input.gst_tax_ids,
        adjustments: input.adjustments,
    })
}

pub async fn insert_payment<C: ConnectionTrait>(
    db: &C,
    new: NewPayment,
) -> Result<payment::Model, String> {
    let now = Utc::now();
    let doc_id = create_source_doc(db, PAYMENT_SOURCE_DOC_TYPE)
        .await
        .map_err(|e| e.to_string())?;
    let (je_id, _) = insert_journal_entry(db, new.datetime, new.journal_id, doc_id, &new.lines)
        .await
        .map_err(|e| e.to_string())?;
    let number = crate::logic::payment_number::resolve_payment_number(
        db,
        new.number.as_deref(),
        new.datetime,
    )
    .await?;
    let pay = payment::ActiveModel {
        customer_id: Set(new.customer_id),
        amount: Set(new.amount),
        datetime: Set(new.datetime),
        number: Set(number.clone()),
        journal_entry_id: Set(je_id),
        is_gst_advance: Set(new.is_gst_advance),
        gst_pos: Set(new.gst_pos.clone()),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("uix_payments_number") {
            format!("payment number {number} is already used")
        } else {
            msg
        }
    })?;
    update_source_doc_id(db, doc_id, pay.id)
        .await
        .map_err(|e| e.to_string())?;
    set_payment_taxes(db, pay.id, &new.withholding_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    crate::logic::tax_assoc::set_payment_gst_taxes(db, pay.id, &new.gst_tax_ids)
        .await
        .map_err(|e| e.to_string())?;
    crate::logic::tax_assoc::replace_advance_adjustments(db, pay.id, &new.adjustments)
        .await
        .map_err(|e| e.to_string())?;
    Ok(pay)
}

pub async fn create_payment(
    db: &DatabaseConnection,
    input: CreatePaymentInput,
) -> Result<payment::Model, String> {
    let new = prepare_payment(db, input).await?;
    let txn = db.begin().await.map_err(|e| e.to_string())?;
    let pay = insert_payment(&txn, new).await?;
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(pay)
}
