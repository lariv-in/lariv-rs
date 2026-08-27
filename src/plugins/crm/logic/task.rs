use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter,
};

use crate::plugins::crm::entities::{
    completed_task::{self, Entity as CompletedTaskEntity},
    task::Entity as TaskEntity,
};
use crate::plugins::crm::scope::find_uncompleted_task;
use crate::plugins::users::state::AuthContext;

pub async fn err_if_task_completed<C: ConnectionTrait>(db: &C, task_id: i64) -> Result<(), String> {
    if task_id == 0 {
        return Ok(());
    }
    let completed = CompletedTaskEntity::find()
        .filter(completed_task::Column::TaskId.eq(task_id))
        .count(db)
        .await
        .map_err(|e| e.to_string())?;
    if completed > 0 {
        return Err("task is completed and cannot be changed".to_string());
    }
    Ok(())
}

pub async fn complete_task(
    db: &sea_orm::DatabaseConnection,
    task_id: i64,
    auth: &AuthContext,
) -> Result<i64, String> {
    let task = find_uncompleted_task(db, task_id, auth)
        .await
        .ok_or_else(|| "task not found or already completed".to_string())?;
    err_if_task_completed(db, task.id).await?;

    let now = Utc::now();
    let row = completed_task::ActiveModel {
        id: Default::default(),
        created_at: Set(Some(now)),
        task_id: Set(task.id),
        completed_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.id)
}

pub async fn completed_task_id_for(db: &sea_orm::DatabaseConnection, task_id: i64) -> Option<i64> {
    crate::web::opt_or_log(
        CompletedTaskEntity::find()
            .filter(completed_task::Column::TaskId.eq(task_id))
            .one(db)
            .await,
        "db find one",
    )
    .map(|c| c.id)
}

pub async fn delete_uncompleted_task(
    db: &sea_orm::DatabaseConnection,
    task_id: i64,
    auth: &AuthContext,
) -> Result<(), String> {
    let existing = find_uncompleted_task(db, task_id, auth)
        .await
        .ok_or_else(|| "task not found or already completed".to_string())?;
    TaskEntity::delete_by_id(existing.id)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
