//! Journal entry item templates for a custom source doc template.
//!
//! Item formulas are Rune expressions. They see the template's variables. The
//! result is a signed journal amount: debit positive, credit negative.

use chrono::Utc;
use lariv_formula::{VariableSchema, VariableValues, eval_formula, parse_values, validate_formula};
use lariv_plugin_finance_common::decimal;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::Deserialize;

use crate::entities::account::{self, Entity as AccountEntity};
use crate::entities::custom_source_doc_journal_entry_item_template::{
    self, Entity as ItemTemplateEntity,
};
use crate::variables;

pub const FORMULA_VARIABLES_HINT: &str = "\
Every variable defined above.
A positive amount is a debit and a negative amount is a credit.";

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
#[expect(
    dead_code,
    reason = "account_label and fk_slot are only carried through the editor JSON round-trip"
)]
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

pub fn validate_item_formula(schema: &VariableSchema, formula: &str) -> Result<(), String> {
    let formula = formula.trim();
    if formula.is_empty() {
        return Err("formula is required".into());
    }
    validate_formula(schema, formula)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn evaluate_item_formula(
    schema_json: &str,
    formula: &str,
    variable_values_json: &str,
) -> Result<Decimal, String> {
    let schema = variables::load_schema(schema_json)?;
    let raw = serde_json::from_str(variable_values_json.trim()).unwrap_or(serde_json::json!({}));
    let values = if schema.is_empty() {
        VariableValues::new()
    } else {
        parse_values(&schema, &raw).map_err(|e| e.to_string())?
    };
    let amount = eval_formula(&schema, formula, &values).map_err(|e| e.to_string())?;
    Ok(decimal::normalize(amount))
}

pub fn evaluate_template_lines(
    schema_json: &str,
    lines: &[ParsedTemplateLine],
    variable_values_json: &str,
) -> Result<Vec<EvaluatedTemplateLine>, String> {
    if lines.is_empty() {
        return Err("at least one journal template line is required".into());
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut balance = Decimal::ZERO;
    for (idx, line) in lines.iter().enumerate() {
        let amount = evaluate_item_formula(schema_json, &line.formula, variable_values_json)
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

pub fn parsed_lines_from_items(
    items: &[custom_source_doc_journal_entry_item_template::Model],
) -> Vec<ParsedTemplateLine> {
    items
        .iter()
        .map(|item| ParsedTemplateLine {
            account_id: item.account_id,
            formula: item.formula.clone(),
        })
        .collect()
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

pub fn validate_parsed_lines(
    schema: &VariableSchema,
    lines: &[ParsedTemplateLine],
) -> Result<(), String> {
    if lines.is_empty() {
        return Err("at least one journal template line is required".into());
    }
    for (idx, line) in lines.iter().enumerate() {
        validate_item_formula(schema, &line.formula)
            .map_err(|e| format!("Line {}: {e}", idx + 1))?;
    }
    Ok(())
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

/// Replace the template's journal lines. Requires at least one valid line.
pub async fn save_template_lines(
    db: &DatabaseConnection,
    template_id: i64,
    lines_json: &str,
    schema: &VariableSchema,
) -> Result<(), String> {
    let lines = parse_template_lines(lines_json)?;
    validate_parsed_lines(schema, &lines)?;
    ensure_leaf_accounts(db, &lines).await?;

    let txn = db.begin().await.map_err(|e| e.to_string())?;
    ItemTemplateEntity::delete_many()
        .filter(
            custom_source_doc_journal_entry_item_template::Column::CustomSourceDocTemplateId
                .eq(template_id),
        )
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    for line in &lines {
        let am = custom_source_doc_journal_entry_item_template::ActiveModel {
            custom_source_doc_template_id: Set(template_id),
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

pub async fn delete_template_lines<C: ConnectionTrait>(
    db: &C,
    template_id: i64,
) -> Result<(), String> {
    ItemTemplateEntity::delete_many()
        .filter(
            custom_source_doc_journal_entry_item_template::Column::CustomSourceDocTemplateId
                .eq(template_id),
        )
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn load_template_lines(
    db: &DatabaseConnection,
    template_id: i64,
) -> Result<Vec<custom_source_doc_journal_entry_item_template::Model>, String> {
    ItemTemplateEntity::find()
        .filter(
            custom_source_doc_journal_entry_item_template::Column::CustomSourceDocTemplateId
                .eq(template_id),
        )
        .order_by_asc(custom_source_doc_journal_entry_item_template::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())
}

pub fn lines_to_json(
    items: &[custom_source_doc_journal_entry_item_template::Model],
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

pub async fn lines_form_json(db: &DatabaseConnection, template_id: i64) -> String {
    let Ok(items) = load_template_lines(db, template_id).await else {
        return empty_lines_json();
    };
    let mut labels = Vec::with_capacity(items.len());
    for item in &items {
        let label = crate::scope::load_account_parent_label(db, Some(item.account_id)).await;
        labels.push(if label == "—" { String::new() } else { label });
    }
    lines_to_json(&items, &labels)
}

pub async fn lines_summary_display(db: &DatabaseConnection, template_id: i64) -> String {
    let Ok(items) = load_template_lines(db, template_id).await else {
        return "—".into();
    };
    if items.is_empty() {
        return "—".into();
    }
    let mut lines = Vec::new();
    for item in &items {
        let account = crate::scope::load_account_parent_label(db, Some(item.account_id)).await;
        lines.push(format!("{account}: {}", item.formula));
    }
    lines.join("\n")
}
