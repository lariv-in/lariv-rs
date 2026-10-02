use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};

use crate::plugins::hr::entities::holiday::{self, Entity as HolidayEntity};

pub struct HolidayInput {
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
}

pub fn validate_holiday_input(input: &HolidayInput) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("title is required".to_string());
    }
    Ok(())
}

pub async fn create_holiday(
    db: &DatabaseConnection,
    input: HolidayInput,
) -> Result<holiday::Model, String> {
    validate_holiday_input(&input)?;
    let now = Utc::now();
    holiday::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        title: Set(input.title.trim().to_string()),
        description: Set(input.description.trim().to_string()),
        date: Set(input.date),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_holiday(
    db: &DatabaseConnection,
    id: i64,
    input: HolidayInput,
) -> Result<holiday::Model, String> {
    validate_holiday_input(&input)?;
    let existing = HolidayEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "holiday not found".to_string())?;
    let now = Utc::now();
    let mut am: holiday::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.title = Set(input.title.trim().to_string());
    am.description = Set(input.description.trim().to_string());
    am.date = Set(input.date);
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_holiday(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    HolidayEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
