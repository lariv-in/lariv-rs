use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};

use crate::entities::attendance::{self, Entity as AttendanceEntity};
use lariv_plugin_users::entities::user::{self, Entity as UserEntity};

pub struct AttendanceInput {
    pub user_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

pub async fn user_display_label(db: &DatabaseConnection, id: i64) -> String {
    if id <= 0 {
        return String::new();
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .map(|user| user.name)
        .unwrap_or_else(|| format!("User {id}"))
}

pub async fn user_display_labels(db: &DatabaseConnection, ids: &[i64]) -> HashMap<i64, String> {
    let ids: Vec<i64> = ids.iter().copied().filter(|id| *id > 0).collect();
    if ids.is_empty() {
        return HashMap::new();
    }
    let users = UserEntity::find()
        .filter(user::Column::Id.is_in(ids))
        .all(db)
        .await
        .unwrap_or_default();
    users.into_iter().map(|user| (user.id, user.name)).collect()
}

async fn user_exists(db: &DatabaseConnection, id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    lariv_core::web::opt_or_log(UserEntity::find_by_id(id).one(db).await, "find user by id")
        .is_some()
}

async fn validate_attendance_input(
    db: &DatabaseConnection,
    input: &AttendanceInput,
) -> Result<(), String> {
    if !user_exists(db, input.user_id).await {
        return Err("user is required".to_string());
    }
    if input.ended_at <= input.started_at {
        return Err("end must be after start".to_string());
    }
    Ok(())
}

pub async fn create_attendance(
    db: &DatabaseConnection,
    input: AttendanceInput,
) -> Result<attendance::Model, String> {
    validate_attendance_input(db, &input).await?;
    let now = Utc::now();
    attendance::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(input.user_id),
        started_at: Set(input.started_at),
        ended_at: Set(Some(input.ended_at)),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

pub async fn update_attendance(
    db: &DatabaseConnection,
    id: i64,
    input: AttendanceInput,
) -> Result<attendance::Model, String> {
    validate_attendance_input(db, &input).await?;
    let existing = AttendanceEntity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "attendance not found".to_string())?;
    let now = Utc::now();
    let mut am: attendance::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.user_id = Set(input.user_id);
    am.started_at = Set(input.started_at);
    am.ended_at = Set(Some(input.ended_at));
    am.update(db).await.map_err(|e| e.to_string())
}

async fn require_employee(db: &DatabaseConnection, user_id: i64) -> Result<(), String> {
    if crate::logic::dashboard::employee_for_user(db, user_id)
        .await
        .is_none()
    {
        return Err("employee is required".to_string());
    }
    Ok(())
}

async fn open_attendance(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<attendance::Model>, String> {
    AttendanceEntity::find()
        .filter(attendance::Column::UserId.eq(user_id))
        .filter(attendance::Column::EndedAt.is_null())
        .order_by_desc(attendance::Column::StartedAt)
        .order_by_desc(attendance::Column::Id)
        .one(db)
        .await
        .map_err(|e| e.to_string())
}

/// Open an attendance row at the current instant. Fails when one is already open.
pub async fn punch_in(db: &DatabaseConnection, user_id: i64) -> Result<attendance::Model, String> {
    require_employee(db, user_id).await?;
    if open_attendance(db, user_id).await?.is_some() {
        return Err("already punched in".to_string());
    }
    let now = Utc::now();
    attendance::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
        user_id: Set(user_id),
        started_at: Set(now),
        ended_at: Set(None),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

/// Close the latest open attendance row at the current instant.
pub async fn punch_out(db: &DatabaseConnection, user_id: i64) -> Result<attendance::Model, String> {
    require_employee(db, user_id).await?;
    let existing = open_attendance(db, user_id)
        .await?
        .ok_or_else(|| "no open attendance".to_string())?;
    let now = Utc::now();
    if now <= existing.started_at {
        return Err("end must be after start".to_string());
    }
    let mut am: attendance::ActiveModel = existing.into();
    am.updated_at = Set(Some(now));
    am.ended_at = Set(Some(now));
    am.update(db).await.map_err(|e| e.to_string())
}

pub async fn delete_attendance(db: &DatabaseConnection, id: i64) -> Result<(), String> {
    AttendanceEntity::delete_by_id(id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
