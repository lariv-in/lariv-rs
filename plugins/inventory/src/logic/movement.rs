use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, TransactionTrait};

use crate::entities::stock_movement::{self, Entity as MovementEntity};
use crate::logic::line::{LineInput, delete_movement_lines, insert_prepared_lines, prepare_lines};
use crate::logic::number::next_movement_number;
use crate::logic::party::BillTo;
use crate::movement_type::MovementType;
use crate::scope::find_movement;

pub struct MovementInput {
    pub number: String,
    pub datetime: DateTime<Utc>,
    pub movement_type: MovementType,
    pub bill_to: BillTo,
    pub vehicle_type: Option<String>,
    pub vehicle_number: Option<String>,
    pub eway_bill: Option<String>,
    pub driver_id: Option<i64>,
    pub lines: Vec<LineInput>,
}

fn optional_text(raw: Option<String>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub async fn create_movement(
    db: &sea_orm::DatabaseConnection,
    input: MovementInput,
) -> Result<stock_movement::Model, String> {
    let prepared = prepare_lines(db, &input.lines).await?;
    let number =
        next_movement_number(db, input.datetime, input.movement_type, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let model = stock_movement::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        datetime: Set(input.datetime),
        movement_type: Set(input.movement_type),
        number: Set(number),
        bill_to_individual: Set(input.bill_to.bill_to_individual),
        customer_individual: Set(input.bill_to.customer_individual),
        customer_company: Set(input.bill_to.customer_company),
        vehicle_type: Set(optional_text(input.vehicle_type)),
        vehicle_number: Set(optional_text(input.vehicle_number)),
        eway_bill: Set(optional_text(input.eway_bill)),
        driver_id: Set(input.driver_id.filter(|id| *id > 0)),
    }
    .insert(&txn)
    .await
    .map_err(|err| err.to_string())?;
    insert_prepared_lines(&txn, model.id, &prepared).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn update_movement(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    input: MovementInput,
) -> Result<stock_movement::Model, String> {
    let existing = find_movement(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    let prepared = prepare_lines(db, &input.lines).await?;
    let number =
        next_movement_number(db, input.datetime, input.movement_type, &input.number).await?;
    let txn = db.begin().await.map_err(|err| err.to_string())?;
    let now = Utc::now();
    let mut am: stock_movement::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.datetime = Set(input.datetime);
    am.movement_type = Set(input.movement_type);
    am.number = Set(number);
    am.bill_to_individual = Set(input.bill_to.bill_to_individual);
    am.customer_individual = Set(input.bill_to.customer_individual);
    am.customer_company = Set(input.bill_to.customer_company);
    am.vehicle_type = Set(optional_text(input.vehicle_type));
    am.vehicle_number = Set(optional_text(input.vehicle_number));
    am.eway_bill = Set(optional_text(input.eway_bill));
    am.driver_id = Set(input.driver_id.filter(|id| *id > 0));
    let model = am.update(&txn).await.map_err(|err| err.to_string())?;
    delete_movement_lines(&txn, model.id).await?;
    insert_prepared_lines(&txn, model.id, &prepared).await?;
    txn.commit().await.map_err(|err| err.to_string())?;
    Ok(model)
}

pub async fn delete_movement(db: &sea_orm::DatabaseConnection, id: i64) -> Result<(), String> {
    let existing = find_movement(db, id)
        .await
        .ok_or_else(|| "movement not found".to_string())?;
    MovementEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
