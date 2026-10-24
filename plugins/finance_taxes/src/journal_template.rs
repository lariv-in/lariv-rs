//! Journal template lines for one tax.
//!
//! Formulas see `percentage` (the number stored on the tax) and `base` (the
//! amount this application uses). The result is a signed journal amount.

use chrono::Utc;
use lariv_formula::{
    Measure, VariableDef, VariableSchema, VariableType, VariableValues, eval_formula,
    validate_formula,
};
use lariv_plugin_finance_accounts::entities::account::{self, Entity as AccountEntity};
use lariv_plugin_finance_accounts::entities::journal::Entity as JournalEntity;
use lariv_plugin_finance_common::decimal;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Deserialize;

use crate::entities::tax::Model as TaxModel;
use crate::entities::tax_journal_entry_item_template::{self, Entity as ItemTemplateEntity};

pub const LINE_BINDINGS: &[&str] = &["percentage", "base"];

/// Variables a tax journal line formula can read.
pub const FORMULA_VARIABLES_HINT: &str = "\
percentage — percentage stored on this tax
base — amount this tax is applied to
A positive amount is a debit and a negative amount is a credit.";

#[derive(Clone, Debug)]
pub struct LoadedTaxTemplate {
    pub journal_id: i64,
    pub items: Vec<tax_journal_entry_item_template::Model>,
}

#[derive(Clone, Debug)]
pub struct ParsedTemplateLine {
    pub account_id: i64,
    pub formula: String,
}

#[derive(Clone, Debug)]
pub struct EvaluatedTemplateLine {
    pub account_id: i64,
    pub amount: Decimal,
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

pub fn validate_item_formula(formula: &str) -> Result<(), String> {
    let formula = formula.trim();
    if formula.is_empty() {
        return Err("formula is required".into());
    }
    validate_formula(&binding_schema(), formula)
        .map(|_| ())
        .map_err(|e| e.to_string())
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
    percentage: Decimal,
    base: Decimal,
) -> Result<Decimal, String> {
    let mut values = VariableValues::new();
    values.insert("percentage".into(), decimal_measure(percentage));
    values.insert("base".into(), decimal_measure(base));
    let amount = eval_formula(&binding_schema(), formula, &values).map_err(|e| e.to_string())?;
    Ok(decimal::normalize(amount))
}

pub fn parse_template_lines(raw: &str) -> Result<Vec<ParsedTemplateLine>, String> {
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

pub fn validate_parsed_lines(lines: &[ParsedTemplateLine]) -> Result<(), String> {
    if lines.is_empty() {
        return Err("at least one journal template line is required".into());
    }
    for (idx, line) in lines.iter().enumerate() {
        validate_item_formula(&line.formula).map_err(|e| format!("Line {}: {e}", idx + 1))?;
    }
    Ok(())
}

pub fn evaluate_template_lines(
    lines: &[ParsedTemplateLine],
    percentage: Decimal,
    base: Decimal,
) -> Result<Vec<EvaluatedTemplateLine>, String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut balance = Decimal::ZERO;
    for (idx, line) in lines.iter().enumerate() {
        let amount = evaluate_item_formula(&line.formula, percentage, base)
            .map_err(|e| format!("Line {}: {e}", idx + 1))?;
        balance = decimal::dec_sum(balance, amount);
        out.push(EvaluatedTemplateLine {
            account_id: line.account_id,
            amount,
        });
    }
    if !decimal::dec_is_zero(balance) {
        return Err("journal template lines do not balance".into());
    }
    Ok(out)
}

async fn ensure_leaf_accounts(
    db: &DatabaseConnection,
    lines: &[ParsedTemplateLine],
) -> Result<(), String> {
    for (idx, line) in lines.iter().enumerate() {
        let n = idx + 1;
        let acct = AccountEntity::find_by_id(line.account_id)
            .one(db)
            .await
            .map_err(|e| format!("Line {n}: {e}"))?
            .ok_or_else(|| format!("Line {n}: account not found"))?;
        if acct.is_group {
            return Err(format!(
                "Line {n}: group accounts cannot be used for posting ({})",
                account_label(&acct)
            ));
        }
    }
    Ok(())
}

fn account_label(acct: &account::Model) -> String {
    if acct.code == 0 {
        acct.name.clone()
    } else {
        format!("{} {}", acct.code, acct.name)
    }
}

pub async fn save_tax_template(
    db: &DatabaseConnection,
    tax_id: i64,
    journal_raw: &str,
    lines_json: &str,
) -> Result<(), String> {
    let journal_id = journal_raw.trim().parse::<i64>().unwrap_or(0);
    let lines = parse_template_lines(lines_json)?;
    if journal_id <= 0 && lines.is_empty() {
        return delete_tax_template(db, tax_id).await;
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
    ensure_leaf_accounts(db, &lines).await?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    ItemTemplateEntity::delete_many()
        .filter(tax_journal_entry_item_template::Column::TaxId.eq(tax_id))
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    for line in &lines {
        let am = tax_journal_entry_item_template::ActiveModel {
            tax_id: Set(tax_id),
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

pub async fn delete_tax_template<C: ConnectionTrait>(db: &C, tax_id: i64) -> Result<(), String> {
    ItemTemplateEntity::delete_many()
        .filter(tax_journal_entry_item_template::Column::TaxId.eq(tax_id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn load_template<C: ConnectionTrait>(
    db: &C,
    tax_id: i64,
) -> Result<Option<LoadedTaxTemplate>, String> {
    let items = ItemTemplateEntity::find()
        .filter(tax_journal_entry_item_template::Column::TaxId.eq(tax_id))
        .order_by_asc(tax_journal_entry_item_template::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let Some(first) = items.first() else {
        return Ok(None);
    };
    let journal_id = first.journal_id;
    if items.iter().any(|item| item.journal_id != journal_id) {
        return Err("tax journal template lines must use the same journal".into());
    }
    Ok(Some(LoadedTaxTemplate { journal_id, items }))
}

pub fn lines_to_json(
    items: &[tax_journal_entry_item_template::Model],
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

/// Every template line. The tax's journal must match the document.
pub async fn evaluate_tax_for_document<C: ConnectionTrait>(
    db: &C,
    tax: &TaxModel,
    base: Decimal,
    expected_journal_id: i64,
) -> Result<Vec<EvaluatedTemplateLine>, String> {
    let Some(template) = load_template(db, tax.id).await? else {
        return Err(format!("{} has no journal entry template", tax.name));
    };
    if template.journal_id != expected_journal_id {
        return Err(format!(
            "{} must use the same journal as the document",
            tax.name
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
    evaluate_template_lines(&parsed, tax.percentage, base).map_err(|e| format!("{}: {e}", tax.name))
}

/// Every withholding template line. These lines are posted next to the payment
/// or purchase entry, which already carries the settlement.
pub async fn evaluate_withholding_lines<C: ConnectionTrait>(
    db: &C,
    tax: &TaxModel,
    base: Decimal,
    expected_journal_id: i64,
) -> Result<Vec<EvaluatedTemplateLine>, String> {
    let Some(template) = load_template(db, tax.id).await? else {
        return Err(format!("{} has no journal entry template", tax.name));
    };
    if template.journal_id != expected_journal_id {
        return Err(format!(
            "{} must use the same journal as the document",
            tax.name
        ));
    }
    let mut out = Vec::with_capacity(template.items.len());
    for (idx, item) in template.items.iter().enumerate() {
        let amount = evaluate_item_formula(&item.formula, tax.percentage, base)
            .map_err(|e| format!("{} line {}: {e}", tax.name, idx + 1))?;
        if decimal::dec_is_zero(amount) {
            continue;
        }
        out.push(EvaluatedTemplateLine {
            account_id: item.account_id,
            amount,
        });
    }
    if out.is_empty() {
        return Err(format!("{} template produced no journal lines", tax.name));
    }
    Ok(out)
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
    fn formula_uses_percentage_and_base() {
        let amount = evaluate_item_formula(
            r#"base * percentage / decimal("100")"#,
            Decimal::from(18),
            Decimal::from(100),
        )
        .unwrap();
        assert_eq!(amount, Decimal::from(18));
    }

    #[test]
    fn template_lines_balance() {
        let lines = vec![
            ParsedTemplateLine {
                account_id: 1,
                formula: r#"base * percentage / decimal("100")"#.into(),
            },
            ParsedTemplateLine {
                account_id: 2,
                formula: r#"decimal("0") - base * percentage / decimal("100")"#.into(),
            },
        ];
        validate_parsed_lines(&lines).unwrap();
        let evaluated =
            evaluate_template_lines(&lines, Decimal::from(18), Decimal::from(200)).unwrap();
        assert_eq!(evaluated[0].amount, Decimal::from(36));
        assert_eq!(evaluated[1].amount, Decimal::from(-36));
    }
}
