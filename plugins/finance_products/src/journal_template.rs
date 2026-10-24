//! Journal template lines for one product.
//!
//! Item formulas are Rune expressions. They see the product's variables plus
//! `quantity`, `rate`, `pre_tax`, `base_cost`, and `sales_price` from the goods
//! line and product. The result is a signed journal amount: debit positive,
//! credit negative, so a credit is written `-pre_tax`. Numeric constants need no
//! `decimal(...)` wrapper and unary minus on a variable is expanded by the
//! formula evaluator.

use chrono::Utc;
use lariv_formula::{
    Measure, VariableDef, VariableSchema, VariableType, VariableValues, eval_formula, parse_values,
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

use crate::entities::product_journal_entry_item_template::{self, Entity as ItemTemplateEntity};
use crate::pricing::load_schema;

#[derive(Clone, Debug)]
pub struct LoadedProductTemplate {
    pub journal_id: i64,
    pub items: Vec<product_journal_entry_item_template::Model>,
}

pub const LINE_BINDINGS: &[&str] = &["quantity", "rate", "pre_tax", "base_cost", "sales_price"];

/// Variables a product journal line formula can read.
pub const FORMULA_VARIABLES_HINT: &str = "\
quantity — invoice line quantity
rate — unit rate on the invoice line
pre_tax — quantity * rate
base_cost — base price of one product
sales_price — sales price of one product
Also every variable defined above.
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

pub fn ensure_schema_allows_bindings(schema: &VariableSchema) -> Result<(), String> {
    for name in LINE_BINDINGS {
        if schema.contains_key(*name) {
            return Err(format!(
                "variable `{name}` is reserved for journal formulas"
            ));
        }
    }
    Ok(())
}

pub fn binding_schema(product: &VariableSchema) -> Result<VariableSchema, String> {
    ensure_schema_allows_bindings(product)?;
    let mut schema = product.clone();
    for name in LINE_BINDINGS {
        schema.insert((*name).to_string(), VariableDef::new(VariableType::Decimal));
    }
    Ok(schema)
}

pub fn validate_item_formula(product: &VariableSchema, formula: &str) -> Result<(), String> {
    let formula = formula.trim();
    if formula.is_empty() {
        return Err("formula is required".into());
    }
    let schema = binding_schema(product)?;
    validate_formula(&schema, formula)
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
    schema_json: &str,
    formula: &str,
    variable_values_json: &str,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
    base_cost: Decimal,
    sales_price: Decimal,
) -> Result<Decimal, String> {
    let product = load_schema(schema_json)?;
    let schema = binding_schema(&product)?;
    let raw = serde_json::from_str(variable_values_json.trim()).unwrap_or(serde_json::json!({}));
    let mut values = if product.is_empty() {
        VariableValues::new()
    } else {
        parse_values(&product, &raw).map_err(|e| e.to_string())?
    };
    values.insert("quantity".into(), decimal_measure(quantity));
    values.insert("rate".into(), decimal_measure(rate));
    values.insert("pre_tax".into(), decimal_measure(pre_tax));
    values.insert("base_cost".into(), decimal_measure(base_cost));
    values.insert("sales_price".into(), decimal_measure(sales_price));
    let amount = eval_formula(&schema, formula, &values).map_err(|e| e.to_string())?;
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

pub fn evaluate_template_lines(
    schema_json: &str,
    lines: &[ParsedTemplateLine],
    variable_values_json: &str,
    quantity: Decimal,
    rate: Decimal,
    pre_tax: Decimal,
    base_cost: Decimal,
    sales_price: Decimal,
) -> Result<Vec<EvaluatedTemplateLine>, String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut balance = Decimal::ZERO;
    for (idx, line) in lines.iter().enumerate() {
        let amount = evaluate_item_formula(
            schema_json,
            &line.formula,
            variable_values_json,
            quantity,
            rate,
            pre_tax,
            base_cost,
            sales_price,
        )
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

/// Save or clear the product's template lines. An empty journal and no lines clears them.
pub async fn save_product_template(
    db: &DatabaseConnection,
    product_id: i64,
    journal_raw: &str,
    lines_json: &str,
    schema: &VariableSchema,
) -> Result<(), String> {
    let journal_id = journal_raw.trim().parse::<i64>().unwrap_or(0);
    let lines = parse_template_lines(lines_json)?;
    if journal_id <= 0 && lines.is_empty() {
        return delete_product_template(db, product_id).await;
    }
    if journal_id <= 0 {
        return Err("journal is required when the template has lines".into());
    }
    if lines.is_empty() {
        return Err("at least one journal template line is required".into());
    }
    validate_parsed_lines(schema, &lines)?;
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
        .filter(product_journal_entry_item_template::Column::ProductId.eq(product_id))
        .exec(&txn)
        .await
        .map_err(|e| e.to_string())?;
    let now = Utc::now();
    for line in &lines {
        let am = product_journal_entry_item_template::ActiveModel {
            product_id: Set(product_id),
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

pub async fn delete_product_template<C: ConnectionTrait>(
    db: &C,
    product_id: i64,
) -> Result<(), String> {
    ItemTemplateEntity::delete_many()
        .filter(product_journal_entry_item_template::Column::ProductId.eq(product_id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn load_template(
    db: &DatabaseConnection,
    product_id: i64,
) -> Result<Option<LoadedProductTemplate>, String> {
    let items = ItemTemplateEntity::find()
        .filter(product_journal_entry_item_template::Column::ProductId.eq(product_id))
        .order_by_asc(product_journal_entry_item_template::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let Some(first) = items.first() else {
        return Ok(None);
    };
    let journal_id = first.journal_id;
    if items.iter().any(|item| item.journal_id != journal_id) {
        return Err("product journal template lines must use the same journal".into());
    }
    Ok(Some(LoadedProductTemplate { journal_id, items }))
}

pub fn lines_to_json(
    items: &[product_journal_entry_item_template::Model],
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pricing::store_schema;

    #[test]
    fn reserved_names_are_rejected() {
        let (_json, schema) = store_schema(&["quantity:decimal".into()]).unwrap();
        let err = ensure_schema_allows_bindings(&schema).unwrap_err();
        assert!(err.contains("quantity"));
    }

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
    fn formula_uses_pre_tax_and_a_product_variable() {
        let (json, schema) = store_schema(&["length:length".into()]).unwrap();
        validate_item_formula(&schema, "pre_tax").unwrap();
        validate_item_formula(&schema, "-pre_tax").unwrap();
        validate_item_formula(&schema, "quantity * length").unwrap();
        validate_item_formula(&schema, "quantity * base_cost").unwrap();
        validate_item_formula(&schema, "-quantity * base_cost").unwrap();
        validate_item_formula(&schema, "quantity * sales_price").unwrap();
        let amount = evaluate_item_formula(
            &json,
            "quantity * length",
            r#"{"length":"1000"}"#,
            Decimal::from(3),
            Decimal::from(10),
            Decimal::from(30),
            Decimal::from(5),
            Decimal::from(10),
        )
        .unwrap();
        assert_eq!(amount, Decimal::from(3000));
    }

    #[test]
    fn template_lines_must_balance() {
        let schema = VariableSchema::new();
        let lines = vec![
            ParsedTemplateLine {
                account_id: 1,
                formula: "pre_tax".into(),
            },
            ParsedTemplateLine {
                account_id: 2,
                formula: "-pre_tax".into(),
            },
        ];
        validate_parsed_lines(&schema, &lines).unwrap();
        let evaluated = evaluate_template_lines(
            "{}",
            &lines,
            "{}",
            Decimal::from(2),
            Decimal::from(10),
            Decimal::from(20),
            Decimal::from(6),
            Decimal::from(10),
        )
        .unwrap();
        assert_eq!(evaluated[0].amount, Decimal::from(20));
        assert_eq!(evaluated[1].amount, Decimal::from(-20));
    }

    #[test]
    fn negated_cost_line_uses_the_product_bindings() {
        let schema = VariableSchema::new();
        let lines = vec![
            ParsedTemplateLine {
                account_id: 1,
                formula: "quantity * base_cost".into(),
            },
            ParsedTemplateLine {
                account_id: 2,
                formula: "-quantity * base_cost".into(),
            },
        ];
        validate_parsed_lines(&schema, &lines).unwrap();
        let evaluated = evaluate_template_lines(
            "{}",
            &lines,
            "{}",
            Decimal::from(3),
            Decimal::from(10),
            Decimal::from(30),
            Decimal::from(5),
            Decimal::from(10),
        )
        .unwrap();
        assert_eq!(evaluated[0].amount, Decimal::from(15));
        assert_eq!(evaluated[1].amount, Decimal::from(-15));
    }

    #[test]
    fn two_goods_lines_balance() {
        let lines = vec![
            ParsedTemplateLine {
                account_id: 1,
                formula: "pre_tax".into(),
            },
            ParsedTemplateLine {
                account_id: 2,
                formula: "-pre_tax".into(),
            },
        ];
        let first = evaluate_template_lines(
            "{}",
            &lines,
            "{}",
            Decimal::from(2),
            Decimal::from(50),
            Decimal::from(100),
            Decimal::from(30),
            Decimal::from(50),
        )
        .unwrap();
        let second = evaluate_template_lines(
            "{}",
            &lines,
            "{}",
            Decimal::from(1),
            Decimal::from(40),
            Decimal::from(40),
            Decimal::from(25),
            Decimal::from(40),
        )
        .unwrap();
        let pre_tax_lines: Decimal = first
            .iter()
            .chain(second.iter())
            .filter(|line| line.account_id == 1)
            .map(|line| line.amount)
            .sum();
        assert_eq!(pre_tax_lines, Decimal::from(140));
        let balance: Decimal = first
            .iter()
            .chain(second.iter())
            .map(|line| line.amount)
            .sum();
        assert!(balance.is_zero());
    }
}
