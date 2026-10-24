//! Shared journal template for invoice receipts.
//!
//! Formulas see `amount` (the settlement) and `withholding` (the withheld tax).
//! The result is a signed journal amount, so a credit is written `-amount`.
//! Numeric constants need no `decimal(...)` wrapper; the formula evaluator
//! expands both.

use chrono::Utc;
use lariv_formula::{
    Measure, VariableDef, VariableSchema, VariableType, VariableValues, eval_formula,
    validate_formula,
};
use lariv_plugin_finance_accounts::entities::account::Entity as AccountEntity;
use lariv_plugin_finance_accounts::entities::journal::Entity as JournalEntity;
use lariv_plugin_finance_accounts::logic::journal::JournalLineSpec;
use lariv_plugin_finance_common::decimal;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Deserialize;

use crate::entities::payment_journal_entry_item_template::{self, Entity as ItemTemplateEntity};

const PREFERENCES_ID: i64 = 1;
pub const LINE_BINDINGS: &[&str] = &["amount", "withholding"];

/// Variables a payment journal line formula can read.
pub const FORMULA_VARIABLES_HINT: &str = "\
amount — settlement on this payment
withholding — withheld tax on this payment
A positive amount is a debit and a negative amount is a credit.";

#[derive(Clone, Debug)]
pub struct LoadedPaymentTemplate {
    pub journal_id: i64,
    pub items: Vec<payment_journal_entry_item_template::Model>,
}

#[derive(Clone, Debug)]
struct ParsedTemplateLine {
    account_id: i64,
    formula: String,
}

#[derive(Clone, Debug, Deserialize)]
#[allow(dead_code)]
struct RawTemplateLine {
    #[serde(default)]
    account_id: i64,
    #[serde(default)]
    formula: String,
    #[serde(default)]
    account_label: String,
    #[serde(default)]
    fk_slot: String,
}

pub fn empty_lines_json() -> String {
    r#"[{"account_id":0,"formula":"","account_label":"","fk_slot":"template-line-0"}]"#.to_string()
}

fn binding_schema() -> VariableSchema {
    let mut schema = VariableSchema::new();
    for name in LINE_BINDINGS {
        schema.insert((*name).to_string(), VariableDef::new(VariableType::Decimal));
    }
    schema
}

fn decimal_measure(amount: Decimal) -> Measure {
    Measure {
        kind: VariableType::Decimal,
        value: amount,
        unit: "None".into(),
    }
}

pub fn evaluate_item_formula(
    formula: &str,
    amount: Decimal,
    withholding: Decimal,
) -> Result<Decimal, String> {
    let mut values = VariableValues::new();
    values.insert("amount".into(), decimal_measure(amount));
    values.insert("withholding".into(), decimal_measure(withholding));
    let result = eval_formula(&binding_schema(), formula, &values).map_err(|e| e.to_string())?;
    Ok(decimal::normalize(result))
}

fn parse_template_lines(raw: &str) -> Result<Vec<ParsedTemplateLine>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<RawTemplateLine> =
        serde_json::from_str(raw).map_err(|_err| "journal template lines could not be read")?;
    let mut lines = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let n = idx + 1;
        let formula = row.formula.trim();
        let blank = row.account_id <= 0 && formula.is_empty();
        if blank {
            continue;
        }
        if row.account_id <= 0 {
            return Err(format!("Line {n}: account is required"));
        }
        if formula.is_empty() {
            return Err(format!("Line {n}: formula is required"));
        }
        lines.push(ParsedTemplateLine {
            account_id: row.account_id,
            formula: formula.to_string(),
        });
    }
    Ok(lines)
}

fn validate_parsed_lines(lines: &[ParsedTemplateLine]) -> Result<(), String> {
    if lines.is_empty() {
        return Err("at least one journal template line is required".into());
    }
    for (idx, line) in lines.iter().enumerate() {
        let formula = line.formula.trim();
        validate_formula(&binding_schema(), formula)
            .map(|_| ())
            .map_err(|e| format!("Line {}: {e}", idx + 1))?;
    }
    Ok(())
}

pub async fn save_payment_template(
    db: &DatabaseConnection,
    journal_raw: &str,
    lines_json: &str,
) -> Result<(), String> {
    let journal_id = journal_raw.trim().parse::<i64>().unwrap_or(0);
    let lines = parse_template_lines(lines_json)?;
    if journal_id <= 0 && lines.is_empty() {
        return delete_payment_template(db).await;
    }
    if journal_id <= 0 {
        return Err("journal is required when the template has lines".into());
    }
    if lines.is_empty() {
        return Err("at least one journal template line is required".into());
    }
    validate_parsed_lines(&lines)?;
    if JournalEntity::find_by_id(journal_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err("journal not found".into());
    }
    for (idx, line) in lines.iter().enumerate() {
        let n = idx + 1;
        let acct = AccountEntity::find_by_id(line.account_id)
            .one(db)
            .await
            .map_err(|e| format!("Line {n}: {e}"))?
            .ok_or_else(|| format!("Line {n}: account not found"))?;
        if acct.is_group {
            let label = if acct.code == 0 {
                acct.name.clone()
            } else {
                format!("{} {}", acct.code, acct.name)
            };
            return Err(format!(
                "Line {n}: group accounts cannot be used for posting ({label})"
            ));
        }
    }

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    delete_payment_template(&txn).await?;
    let now = Utc::now();
    for line in &lines {
        let am = payment_journal_entry_item_template::ActiveModel {
            payment_preferences_id: Set(PREFERENCES_ID),
            journal_id: Set(journal_id),
            account_id: Set(line.account_id),
            formula: Set(line.formula.clone()),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        };
        am.insert(&txn).await.map_err(|e| e.to_string())?;
    }
    txn.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_payment_template<C: ConnectionTrait>(db: &C) -> Result<(), String> {
    ItemTemplateEntity::delete_many()
        .filter(
            payment_journal_entry_item_template::Column::PaymentPreferencesId.eq(PREFERENCES_ID),
        )
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn load_template<C: ConnectionTrait>(
    db: &C,
) -> Result<Option<LoadedPaymentTemplate>, String> {
    let items = ItemTemplateEntity::find()
        .filter(
            payment_journal_entry_item_template::Column::PaymentPreferencesId.eq(PREFERENCES_ID),
        )
        .order_by_asc(payment_journal_entry_item_template::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let Some(first) = items.first() else {
        return Ok(None);
    };
    let journal_id = first.journal_id;
    if items.iter().any(|item| item.journal_id != journal_id) {
        return Err("payment journal template lines must use the same journal".into());
    }
    Ok(Some(LoadedPaymentTemplate { journal_id, items }))
}

pub fn lines_to_json(
    items: &[payment_journal_entry_item_template::Model],
    labels: &[String],
) -> String {
    let rows: Vec<serde_json::Value> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            serde_json::json!({
                "account_id": item.account_id,
                "formula": item.formula,
                "account_label": labels.get(i).cloned().unwrap_or_default(),
                "fk_slot": format!("template-line-{i}"),
            })
        })
        .collect();
    if rows.is_empty() {
        empty_lines_json()
    } else {
        serde_json::to_string(&rows).unwrap_or_else(|_| empty_lines_json())
    }
}

/// Payment template lines for one allocation. Withholding tax lines are separate.
pub async fn evaluate_payment_template<C: ConnectionTrait>(
    db: &C,
    amount: Decimal,
    withholding: Decimal,
    expected_journal_id: i64,
) -> Result<Vec<JournalLineSpec>, String> {
    let Some(template) = load_template(db).await? else {
        return Err("payment preferences have no journal entry template".into());
    };
    if template.journal_id != expected_journal_id {
        return Err("payment template must use the same journal as the invoice".into());
    }
    let mut lines = Vec::with_capacity(template.items.len());
    for (idx, item) in template.items.iter().enumerate() {
        let value = evaluate_item_formula(&item.formula, amount, withholding)
            .map_err(|e| format!("payment template line {}: {e}", idx + 1))?;
        lines.push(JournalLineSpec {
            account_id: item.account_id,
            amount: value,
        });
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_hint_names_every_injected_variable() {
        for name in LINE_BINDINGS {
            assert!(
                FORMULA_VARIABLES_HINT
                    .lines()
                    .any(|line| line.starts_with(&format!("{name} —"))),
                "hint missing {name}"
            );
        }
    }

    #[test]
    fn payment_lines_balance_with_and_without_withholding() {
        for (amount, withholding) in [
            (Decimal::from(100), Decimal::ZERO),
            (Decimal::from(100), Decimal::from(10)),
        ] {
            let bank = evaluate_item_formula("amount - withholding", amount, withholding).unwrap();
            let credit =
                evaluate_item_formula("withholding - amount", amount, withholding).unwrap();
            assert_eq!(bank + credit, Decimal::ZERO);
        }
    }
}
