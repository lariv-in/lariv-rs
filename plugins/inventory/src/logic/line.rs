use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
};
use serde::Deserialize;

use crate::entities::stock_movement_line::{self, Entity as LineEntity};
use crate::logic::qty::{check_line, parse_qty};
use crate::scope::{find_stock, stock_names};

pub struct LineInput {
    pub stock_id: i64,
    pub qty: Decimal,
    pub qty_unit: String,
    pub qty_type: String,
}

pub struct PreparedLine {
    pub stock_id: i64,
    pub qty: Decimal,
    pub qty_unit: String,
    pub qty_type: String,
}

#[derive(Deserialize)]
struct RawLine {
    #[serde(default)]
    stock_id: i64,
    #[serde(default)]
    qty: String,
    #[serde(default)]
    qty_unit: String,
    #[serde(default)]
    qty_type: String,
}

pub fn default_movement_lines_json() -> String {
    r#"[{"stock_id":0,"stock_label":"","fk_slot":"MovementLineStock_0","qty":"","qty_unit":"","qty_type":""}]"#
        .to_string()
}

/// Parse the movement form's line JSON. Blank rows are skipped.
pub fn parse_movement_lines_json(raw: &str) -> Result<Vec<LineInput>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("add at least one line".into());
    }
    let rows: Vec<RawLine> =
        serde_json::from_str(raw).map_err(|_err| "lines are invalid".to_string())?;
    let mut out = Vec::new();
    for row in rows {
        if row.stock_id <= 0 && row.qty.trim().is_empty() {
            continue;
        }
        let qty = parse_qty(&row.qty)?;
        out.push(LineInput {
            stock_id: row.stock_id,
            qty,
            qty_unit: row.qty_unit,
            qty_type: row.qty_type,
        });
    }
    if out.is_empty() {
        return Err("add at least one line".into());
    }
    Ok(out)
}

pub async fn movement_lines_form_json(
    db: &sea_orm::DatabaseConnection,
    movement_id: i64,
) -> String {
    let models = LineEntity::find()
        .filter(stock_movement_line::Column::StockMovementId.eq(movement_id))
        .all(db)
        .await
        .unwrap_or_default();
    if models.is_empty() {
        return default_movement_lines_json();
    }
    let names = stock_names(
        db,
        &models.iter().map(|line| line.stock_id).collect::<Vec<_>>(),
    )
    .await;
    let rows: Vec<serde_json::Value> = models
        .into_iter()
        .map(|line| {
            serde_json::json!({
                "stock_id": line.stock_id,
                "stock_label": names.get(&line.stock_id).cloned().unwrap_or_default(),
                "fk_slot": format!("MovementLineStock_{}", line.id),
                "qty": line.qty.normalize().to_string(),
                "qty_unit": line.qty_unit,
                "qty_type": line.qty_type,
            })
        })
        .collect();
    serde_json::to_string(&rows).unwrap_or_else(|_| default_movement_lines_json())
}

pub async fn prepare_lines(
    db: &sea_orm::DatabaseConnection,
    lines: &[LineInput],
) -> Result<Vec<PreparedLine>, String> {
    if lines.is_empty() {
        return Err("add at least one line".into());
    }
    let mut prepared = Vec::with_capacity(lines.len());
    for input in lines {
        prepared.push(prepare_line(db, input).await?);
    }
    Ok(prepared)
}

pub async fn insert_prepared_lines<C: ConnectionTrait>(
    db: &C,
    movement_id: i64,
    lines: &[PreparedLine],
) -> Result<(), String> {
    let now = Utc::now();
    for line in lines {
        stock_movement_line::ActiveModel {
            id: Default::default(),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            stock_id: Set(line.stock_id),
            stock_movement_id: Set(movement_id),
            qty: Set(line.qty),
            qty_unit: Set(line.qty_unit.clone()),
            qty_type: Set(line.qty_type.clone()),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub async fn delete_movement_lines<C: ConnectionTrait>(
    db: &C,
    movement_id: i64,
) -> Result<(), String> {
    LineEntity::delete_many()
        .filter(stock_movement_line::Column::StockMovementId.eq(movement_id))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

async fn prepare_line(
    db: &sea_orm::DatabaseConnection,
    input: &LineInput,
) -> Result<PreparedLine, String> {
    if input.stock_id <= 0 {
        return Err("stock is required".into());
    }
    let stock = find_stock(db, input.stock_id)
        .await
        .ok_or_else(|| "stock not found".to_string())?;
    let (qty_type, qty_unit) = check_line(
        &stock.qty_type,
        &stock.qty_unit,
        input.qty,
        &input.qty_unit,
        &input.qty_type,
    )?;
    Ok(PreparedLine {
        stock_id: stock.id,
        qty: input.qty.round_dp(6),
        qty_unit,
        qty_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skips_blank_rows_and_keeps_units() {
        let lines = parse_movement_lines_json(
            r#"[{"stock_id":0,"qty":""},{"stock_id":4,"qty":"1.5","qty_unit":"g","qty_type":"weight"}]"#,
        )
        .expect("lines");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].stock_id, 4);
        assert_eq!(lines[0].qty_unit, "g");
        assert_eq!(lines[0].qty_type, "weight");
    }

    #[test]
    fn parse_requires_a_line() {
        match parse_movement_lines_json(r#"[{"stock_id":0,"qty":""}]"#) {
            Err(err) => assert!(err.contains("at least one")),
            Ok(_) => panic!("blank lines should be rejected"),
        }
    }
}
