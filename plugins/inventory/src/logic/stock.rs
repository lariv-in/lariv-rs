use chrono::Utc;
use lariv_plugin_contacts::entities::company::Entity as CompanyEntity;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};

use crate::entities::stock::{self, Entity as StockEntity};
use crate::entities::stock_movement_line::{self, Entity as LineEntity};
use crate::logic::qty::{parse_type_name, stock_display_unit, type_uses_unit};
use crate::scope::find_stock;

pub struct StockInput {
    pub name: String,
    pub company_id: i64,
    pub qty_type: String,
    pub qty_unit: String,
}

pub async fn create_stock(
    db: &sea_orm::DatabaseConnection,
    input: StockInput,
) -> Result<stock::Model, String> {
    let prepared = prepare_stock(db, &input).await?;
    let now = Utc::now();
    let model = stock::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        name: Set(prepared.name),
        company_id: Set(prepared.company_id),
        qty_type: Set(prepared.qty_type),
        qty_unit: Set(prepared.qty_unit),
    };
    model.insert(db).await.map_err(|err| err.to_string())
}

pub async fn update_stock(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    input: StockInput,
) -> Result<stock::Model, String> {
    let existing = find_stock(db, id)
        .await
        .ok_or_else(|| "stock not found".to_string())?;
    let mut prepared = prepare_stock(db, &input).await?;
    if stock_has_lines(db, id).await? {
        let existing_ty = parse_type_name(&existing.qty_type)?;
        let next_ty = parse_type_name(&prepared.qty_type)?;
        let unit_changed = type_uses_unit(existing_ty) && prepared.qty_unit != existing.qty_unit;
        if existing_ty != next_ty || unit_changed {
            return Err("quantity type and unit cannot change while movements exist".into());
        }
        if !type_uses_unit(existing_ty) {
            prepared.qty_unit = existing.qty_unit.clone();
        }
    }
    let now = Utc::now();
    let mut am: stock::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.name = Set(prepared.name);
    am.company_id = Set(prepared.company_id);
    am.qty_type = Set(prepared.qty_type);
    am.qty_unit = Set(prepared.qty_unit);
    am.update(db).await.map_err(|err| err.to_string())
}

pub async fn delete_stock(db: &sea_orm::DatabaseConnection, id: i64) -> Result<(), String> {
    let existing = find_stock(db, id)
        .await
        .ok_or_else(|| "stock not found".to_string())?;
    if stock_has_lines(db, id).await? {
        return Err("stock still has movement lines".into());
    }
    StockEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

struct PreparedStock {
    name: String,
    company_id: i64,
    qty_type: String,
    qty_unit: String,
}

async fn prepare_stock(
    db: &sea_orm::DatabaseConnection,
    input: &StockInput,
) -> Result<PreparedStock, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("name is required".into());
    }
    if input.company_id <= 0 {
        return Err("company is required".into());
    }
    let company = CompanyEntity::find_by_id(input.company_id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if company.is_none() {
        return Err("company not found".into());
    }
    let ty = parse_type_name(&input.qty_type)?;
    let qty_unit = stock_display_unit(ty, &input.qty_unit)?;
    Ok(PreparedStock {
        name,
        company_id: input.company_id,
        qty_type: ty.as_str().to_string(),
        qty_unit,
    })
}

async fn stock_has_lines(db: &sea_orm::DatabaseConnection, stock_id: i64) -> Result<bool, String> {
    let row = LineEntity::find()
        .filter(stock_movement_line::Column::StockId.eq(stock_id))
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(row.is_some())
}
